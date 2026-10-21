use crate::{auth::SessionState, commands::auditoria, db::DbConnection, payments::{legacy_payment, Breakdown}};
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

type DbConn<'a> = State<'a, DbConnection>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CorteCaja {
    id: i32,
    fecha: String,
    total_ventas: f64,
    esperado_efectivo: f64,
    esperado_tarjeta: f64,
    contado_efectivo: f64,
    contado_tarjeta: f64,
    diferencia: f64,
    usuario: String,
    fecha_hora_cierre: String,
}

#[derive(serde::Serialize)]
pub struct ResumenCorte {
    fecha: String,
    total_ventas: f64,
    esperado_efectivo: f64,
    esperado_tarjeta: f64,
    corte_existente: Option<CorteCaja>,
}

fn clean_date(date: &str) -> Result<String, String> {
    let cleaned = date.replace(' ', "");
    NaiveDate::parse_from_str(&cleaned, "%Y-%m-%d")
        .map_err(|_| "Fecha inválida; usa AAAA-MM-DD".to_string())?;
    Ok(cleaned)
}

pub fn totals(conn: &Connection, date: &str) -> Result<(f64, f64, f64), String> {
    let mut gross = 0.0;
    let mut cash = 0.0;
    let mut card = 0.0;
    let mut statement = conn.prepare(
        "SELECT metodo_pago, total_venta, pago_efectivo, pago_tarjeta FROM ventas WHERE substr(fecha_hora, 1, 10) = ?1",
    ).map_err(|e| e.to_string())?;
    let sales = statement.query_map(params![date], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?,
            row.get::<_, Option<f64>>(2)?, row.get::<_, Option<f64>>(3)?))
    }).map_err(|e| e.to_string())?;
    for sale in sales {
        let (method, total, cash_saved, card_saved) = sale.map_err(|e| e.to_string())?;
        let breakdown = match (cash_saved, card_saved) {
            (Some(cash), Some(card)) => Breakdown { cash, card },
            _ => legacy_payment(&method, total)?,
        };
        gross += total;
        cash += breakdown.cash;
        card += breakdown.card;
    }

    let mut refunds = conn.prepare(
        "SELECT d.total_reembolsado, d.reembolso_efectivo, d.reembolso_tarjeta,
                v.total_venta, v.metodo_pago, v.pago_efectivo, v.pago_tarjeta
         FROM devoluciones d JOIN ventas v ON v.id = d.id_venta_original
         WHERE substr(d.fecha_hora, 1, 10) = ?1",
    ).map_err(|e| e.to_string())?;
    let rows = refunds.query_map(params![date], |row| {
        Ok((row.get::<_, f64>(0)?, row.get::<_, Option<f64>>(1)?,
            row.get::<_, Option<f64>>(2)?, row.get::<_, f64>(3)?,
            row.get::<_, String>(4)?, row.get::<_, Option<f64>>(5)?,
            row.get::<_, Option<f64>>(6)?))
    }).map_err(|e| e.to_string())?;
    for row in rows {
        let (refund, refund_cash, refund_card, sale_total, method, sale_cash, sale_card) =
            row.map_err(|e| e.to_string())?;
        let (cash_amount, card_amount) = match (refund_cash, refund_card) {
            (Some(cash), Some(card)) => (cash, card),
            _ => {
                let payment = match (sale_cash, sale_card) {
                    (Some(cash), Some(card)) => Breakdown { cash, card },
                    _ => legacy_payment(&method, sale_total)?,
                };
                if sale_total <= 0.0 {
                    return Err("Venta original sin importe para distribuir reembolso".into());
                }
                let cash_amount = (refund * payment.cash / sale_total * 100.0).round() / 100.0;
                (cash_amount, refund - cash_amount)
            }
        };
        cash -= cash_amount;
        card -= card_amount;
    }
    Ok((gross, cash, card))
}

#[tauri::command]
pub fn obtener_resumen_corte(db_state: State<DbConnection>, fecha: String) -> Result<ResumenCorte, String> {
    let fecha = clean_date(&fecha)?;
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let (total_ventas, esperado_efectivo, esperado_tarjeta) = totals(&conn, &fecha)?;
    let corte_existente = conn.query_row(
        "SELECT id, fecha, total_ventas, esperado_efectivo, esperado_tarjeta, contado_efectivo, contado_tarjeta, diferencia, usuario, fecha_hora_cierre FROM cortes_caja WHERE REPLACE(fecha, ' ', '') = ?1",
        params![fecha],
        |row| Ok(CorteCaja {
            id: row.get(0)?, fecha: row.get(1)?, total_ventas: row.get(2)?,
            esperado_efectivo: row.get(3)?, esperado_tarjeta: row.get(4)?,
            contado_efectivo: row.get(5)?, contado_tarjeta: row.get(6)?,
            diferencia: row.get(7)?, usuario: row.get(8)?, fecha_hora_cierre: row.get(9)?,
        }),
    ).optional().map_err(|e| e.to_string())?;
    Ok(ResumenCorte { fecha, total_ventas, esperado_efectivo, esperado_tarjeta, corte_existente })
}

#[tauri::command]
pub fn guardar_corte(
    db_state: State<DbConnection>,
    fecha: String,
    contado_efectivo: f64,
    contado_tarjeta: f64,
    session: State<SessionState>,
) -> Result<(), String> {
    if !contado_efectivo.is_finite() || !contado_tarjeta.is_finite()
        || contado_efectivo < 0.0 || contado_tarjeta < 0.0 {
        return Err("Los montos contados deben ser válidos y positivos".into());
    }
    let fecha = clean_date(&fecha)?;
    let username = session.current_user()?.username;
    let mut conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let (total_ventas, esperado_efectivo, esperado_tarjeta) = totals(&conn, &fecha)?;
    let difference = (((contado_efectivo + contado_tarjeta) - (esperado_efectivo + esperado_tarjeta)) * 100.0).round() / 100.0;
    let now = chrono::Local::now().to_rfc3339();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO cortes_caja (fecha, total_ventas, esperado_efectivo, esperado_tarjeta, contado_efectivo, contado_tarjeta, diferencia, usuario, fecha_hora_cierre) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![fecha, total_ventas, esperado_efectivo, esperado_tarjeta,
            contado_efectivo, contado_tarjeta, difference, username, now],
    ).map_err(|e| e.to_string())?;
    let details = format!("fecha={fecha}; diferencia={difference:.2}");
    auditoria::registrar_evento_interno(&tx, &username, "CORTE_CAJA", Some(&details))
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn eliminar_corte(fecha: String, db: DbConn) -> Result<(), String> {
    let fecha = clean_date(&fecha)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM cortes_caja WHERE REPLACE(fecha, ' ', '') = ?1", params![fecha])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refunds_reduce_cash_and_card_totals() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::inicializar(&conn).unwrap();
        conn.execute(
            "INSERT INTO ventas (fecha_hora, total_venta, metodo_pago, pago_efectivo, pago_tarjeta) VALUES ('2026-10-19T10:00:00-06:00', 100, 'Mixto', 60, 40)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO devoluciones (id_venta_original, fecha_hora, total_reembolsado, reembolso_efectivo, reembolso_tarjeta) VALUES (1, '2026-10-19T11:00:00-06:00', 50, 30, 20)",
            [],
        ).unwrap();
        assert_eq!(totals(&conn, "2026-10-19").unwrap(), (100.0, 30.0, 20.0));
    }
}
