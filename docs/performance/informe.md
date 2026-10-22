# Medición de CPU — 21 de octubre de 2026

Versión Release instalada, Windows 11 Pro 10.0.26200, 12 procesadores lógicos. Se sumaron los tiempos de CPU de calendar-app y sus siete procesos descendientes de WebView2. Porcentajes normalizados sobre la capacidad total del equipo.

| Escenario | Duración | CPU media | Mayor intervalo |
| --- | ---: | ---: | ---: |
| Sin interacción, widget visible | 30,673 s | 0,0085 % | 0,0540 % |
| Desplegar y plegar el widget | 30,334 s | 0,3477 % | 1,2203 % |

Los intervalos son aproximadamente de 2–3 segundos; no son picos instantáneos. La prueba activa contiene pausas entre acciones y no representa interacción continua. Agenda vacía. No se midieron GPU, energía, arranque, notificaciones ni una agenda grande. Estos resultados indican CPU baja en las condiciones probadas, sin garantizar el mismo consumo en otros equipos.

Datos: idle-widget.json/csv y active-ui.json/csv. Script: scripts/measure-calendar.ps1 desde la raíz del proyecto. Las primeras capturas descartadas redondeaban incorrectamente diferencias pequeñas por la sobrecarga entera de Math.Max; los archivos actuales fueron regenerados con precisión decimal.

Windows 10 x64 debería ser compatible, pero no se ha probado en ese sistema. Tauri usa WebView2 en Windows; el instalador generado tiene INSTALLWEBVIEW2MODE downloadBootstrapper y descarga el runtime si falta, por lo que en ese caso necesita Internet. Referencia: https://v2.tauri.app/distribute/windows-installer/

