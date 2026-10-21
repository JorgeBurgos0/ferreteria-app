use regex::Regex;

#[derive(Clone, Copy, Debug)]
pub struct Breakdown {
    pub cash: f64,
    pub card: f64,
}

pub fn validate_payment(method: &str, cash: f64, card: f64, total: f64) -> Result<Breakdown, String> {
    if [cash, card, total].iter().any(|amount| !amount.is_finite() || *amount < 0.0)
        || (cash + card - total).abs() > 0.01
    {
        return Err("Los importes de pago no coinciden con el total".into());
    }
    match method {
        "Efectivo" if card.abs() < 0.001 => Ok(Breakdown { cash, card: 0.0 }),
        "Tarjeta" if cash.abs() < 0.001 => Ok(Breakdown { cash: 0.0, card }),
        "Mixto" if cash > 0.0 && card > 0.0 => Ok(Breakdown { cash, card }),
        _ => Err("Método de pago inválido".into()),
    }
}

pub fn legacy_payment(method: &str, total: f64) -> Result<Breakdown, String> {
    if method.starts_with("Efectivo") {
        return Ok(Breakdown { cash: total, card: 0.0 });
    }
    if method.starts_with("Tarjeta") {
        return Ok(Breakdown { cash: 0.0, card: total });
    }
    let pattern = Regex::new(r"^Mixto \(Efec: \$([0-9]+(?:\.[0-9]+)?), Tarj: \$([0-9]+(?:\.[0-9]+)?)\)$")
        .map_err(|e| e.to_string())?;
    let captures = pattern.captures(method).ok_or("Pago histórico no reconocido")?;
    let cash: f64 = captures[1].parse().map_err(|_| "Pago histórico inválido")?;
    let card: f64 = captures[2].parse().map_err(|_| "Pago histórico inválido")?;
    if (cash + card - total).abs() > 0.01 {
        return Err("El desglose del pago histórico no coincide con la venta".into());
    }
    Ok(Breakdown { cash, card })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_inconsistent_payment() {
        assert!(validate_payment("Mixto", 10.0, 30.0, 50.0).is_err());
        assert!(validate_payment("Tarjeta", 50.0, 0.0, 50.0).is_err());
    }

    #[test]
    fn reads_legacy_mixed_payment() {
        let payment = legacy_payment("Mixto (Efec: $30.00, Tarj: $20.00)", 50.0).unwrap();
        assert_eq!(payment.cash, 30.0);
        assert_eq!(payment.card, 20.0);
    }
}
