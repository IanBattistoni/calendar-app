# Calendario para Windows 11

Aplicación local ligera con Tauri 2, Rust, TypeScript y Vite. El diseño de Claude se utiliza como referencia visual. No requiere React, SQLite, cuentas ni servidor de producción.

## Funciones

- Calendario mensual, navegación por teclado, creación, edición y eliminación de eventos, cumpleaños anuales y recordatorios.
- Tema claro, oscuro o automático; color, tamaño, densidad y primer día configurables.
- Icono permanente en la bandeja mientras el proceso está activo. Cerrar el calendario oculta la ventana. Una segunda instancia abre la existente.
- El calendario abre en la esquina inferior derecha del área útil del monitor, incluso desde los iconos ocultos de Windows.
- Widget opcional con agenda o próximo evento. Al pasar el mouse se expande con próximos avisos y eventos, fecha y cuenta regresiva; al retirarlo recupera su tamaño. Configuración de expansión, cantidad, esquina y mantener encima.
- Animación nativa breve, limitada al cambio de tamaño. La expansión se ajusta al área útil y crece hacia arriba cuando no cabe debajo.
- Avisos de Windows con anticipación configurable por evento. Los días completos tienen hora de aviso; los cumpleaños admiten varios días de anticipación.
- Configuración global de avisos, recuperación del aviso más reciente pendiente de cada evento y botón de prueba.

## Persistencia y avisos

Rust guarda events.json, preferences.json y notification-history.json en los directorios de datos/configuración de la aplicación. Desactivar el widget destruye su WebView y conserva los datos. Los eventos existentes sin avisos configurados conservan esa condición.

El planificador nativo espera hasta el próximo vencimiento y se despierta al editar datos, cambiar preferencias o recibir señales de cambio de reloj y reanudación. Los avisos usan hora local; los anticipos de días completos conservan la hora civil ante cambios de horario. Las señales de Windows también actualizan las vistas.

El historial se guarda antes de entregar el aviso a Windows para evitar duplicados al reiniciar. Un cierre abrupto entre ese guardado y la entrega puede omitir ese aviso. Un error explícito de envío permite reintentar. La aceptación del envío por la API no garantiza su presentación: Windows controla No molestar y las preferencias de notificaciones.

Los avisos requieren que Calendario siga ejecutándose en la bandeja. Salir de la aplicación termina el planificador. Al iniciar o reanudar recupera avisos según la ventana elegida (sin recuperación, una hora o 24 horas).

## Desarrollo y comprobaciones

Requiere Rust MSVC, herramientas C++ de Visual Studio y WebView2. Si cargo no está en PATH, añadir la carpeta .cargo/bin del usuario.

```powershell
npm install
npm run tauri dev
```

```powershell
npm test
npm run build
cargo test --offline --manifest-path src-tauri/Cargo.toml
cargo fmt --check --manifest-path src-tauri/Cargo.toml
```

Cargo offline requiere dependencias descargadas. La vista previa del navegador no crea bandeja, avisos nativos ni persistencia.

Verificación de esta etapa: diez pruebas frontend y trece Rust (fechas, escape de contenido, cuenta regresiva, validación/migración, avisos y geometría). Compilación frontend y paquete NSIS Debug correctos. El usuario confirmó la aparición real de una notificación de la versión instalada. Se verificaron guardado de evento y expansión/contracción nativa del widget. La tarjeta compacta y la agenda usan diseños separados; las capturas anteriores a esta corrección son históricas. El evento temporal se retiró conservando los demás datos.

El instalador de prueba está en src-tauri/target/debug/bundle/nsis/Calendario_0.1.0_x64-setup.exe y pesa aproximadamente 2,75 MiB. Es una compilación Debug para verificación; aún falta validar Release, firma y distribución final. No se ha completado la matriz de monitores/DPI, suspensión física y cambios reales de zona horaria.

## Estructura

- src/calendar.ts: fechas civiles y cuadrícula.
- src/events.ts y src/event-editor.ts: contratos, agenda y formularios.
- src/widget.ts: interacción de expansión; src/widget-data.ts: próximos eventos y cuenta regresiva.
- src/main.ts y src/styles.css: vistas, configuración y estilos.
- src-tauri/src/events.rs: CRUD y ocurrencias anuales; 29 de febrero pasa al 28 en años no bisiestos.
- src-tauri/src/preferences.rs: preferencias persistentes y validación.
- src-tauri/src/reminders.rs: planificación, historial y entrega de avisos.
- src-tauri/src/system_events.rs: señales nativas de reloj y reanudación.
- src-tauri/src/windows.rs: ventanas, posiciones y animación del widget.

## Rendimiento y pendientes

No hay sondeo periódico para buscar avisos ni animaciones permanentes. La cuenta regresiva se actualiza una vez por minuto únicamente cuando el widget está visible y tiene contenido. El widget desactivado no mantiene WebView. Las fuentes son del sistema y los archivos web se incluyen compilados.

Release configura LTO, strip, una unidad de generación y opt-level=s. Queda medir RAM de todo el árbol WebView2, CPU inactiva y latencia en Release; el tamaño del frontend no representa esos consumos.

Pendientes: inicio automático opcional, atajos, importación/exportación, recurrencias adicionales y paquete final para distribución. JSON cubre el alcance actual; SQLite no es obligatorio. El widget es una ventana normal y aún no está integrado en la capa del escritorio de Explorer. Conserva la esquina seleccionada; la posición libre arrastrada no persiste entre reinicios. Windows decide si el icono de bandeja aparece junto al reloj o dentro de los iconos ocultos.

Paquete Release optimizado generado correctamente: entregas/Calendario_0.1.0_x64-setup.exe (1,28 MiB), Windows x64, sin firma digital. La prueba nativa de notificaciones se realizó sobre Debug; falta probar el instalador Release en otro equipo.

Correcci�n del widget: modos separados, selecci�n predeterminada Pr�ximo evento para instalaciones nuevas, navegaci�n semanal, tarjeta sin duplicar el primer evento, arrastre coordinado con Windows y conservaci�n del desplazamiento al contraerse. Pruebas de entradas/salidas r�pidas, bloqueo durante arrastre, recuperaci�n ante error y geometr�a tras mover. Icono de bandeja redibujado como hoja de calendario con cuadr�cula.


Verificación posterior: tarjeta instalada Release arranca en 352×106 píxeles; agenda fija en 422×502 en este equipo. Se verificó apertura del editor desde la tarjeta en la ventana principal. El arrastre está coordinado con WM_ENTERSIZEMOVE/WM_EXITSIZEMOVE y probado en lógica/geometría, pero la herramienta no reprodujo movimiento sostenido, por lo que falta confirmación manual del desplazamiento y ausencia de parpadeo durante una sesión real. Los dos eventos temporales se retiraron por identificador conservando los demás datos. Installer entregas actualizado (1,28 MiB).


Corrección de foco y monitores: mostrar el widget utiliza SW_SHOWNOACTIVATE y conserva abierto el calendario. La preferencia de ocultación al hacer clic fuera se aplica tanto al calendario como a Configuración, respetando formularios abiertos y popups nativos del propio panel. Se revalida una vez el foco tras 100 ms; no hay sondeo permanente. El tamaño y la posición del panel se calculan sobre un único monitor antes de cambiar sus límites, y se aplican mediante SetWindowPos en una operación. La ventana inicial permanece oculta hasta quedar colocada. Cambiar opciones del widget ya no redimensiona innecesariamente el calendario.

Verificación: 23 tests (10 frontend y 13 Rust). En Windows se confirmó que activar el widget conserva visibles ambas ventanas, que el selector de tema no oculta Configuración y que un clic en el widget sí oculta Configuración. Calendario y Configuración conservaron el mismo borde inferior y derecho al cambiar de modo. Las capturas puntuales no permiten certificar un parpadeo de un milisegundo; se eliminó la secuencia intermedia de redimensionar y después mover que lo podía provocar.

Sombra del widget: se desactiva la sombra nativa y se dibuja una sombra propia al 6 % en tema claro y 14 % en oscuro (mitad de los valores 12 %/28 % del diseño de referencia). La reducción respecto a la sombra de Windows es aproximada, porque el sistema no expone una intensidad graduable. Se reserva un margen transparente de 12 px por lado y se mantienen separadas las dimensiones del contenido y las de la ventana. El marco nativo se recalcula antes de medir para no añadir insets antiguos. Se comprobó visualmente la transparencia y la sombra en Windows.

Corrección de despliegue y padding: la altura compacta se obtiene midiendo el encabezado, su padding simétrico, bordes y margen transparente en el DOM; Rust convierte usando la relación real entre el viewport CSS y el área cliente. No se usa una altura fija para ajustar el contenido. El despliegue conserva x/y mediante SWP_NOMOVE; si no cabe debajo, se limita el alto y la lista permite desplazamiento, sin subir el encabezado. Se confirmó en Windows que pasa de 135 a 485 px conservando x=2129, y=4 y ancho=427 en el equipo de prueba. Pruebas: 11 frontend y 12 Rust.
