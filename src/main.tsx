import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";
import '@mantine/core/styles.css';
import '@mantine/charts/styles.css';
import '@mantine/notifications/styles.css';
import '@mantine/dates/styles.css';
import { MantineProvider } from '@mantine/core';
import { Notifications } from '@mantine/notifications';

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MantineProvider>
      <Notifications />
      <App />
    </MantineProvider>
  </React.StrictMode>
);