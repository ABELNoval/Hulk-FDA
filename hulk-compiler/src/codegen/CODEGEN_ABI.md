**Hulk Compiler — Codegen ABI & Calling Conventions**

Resumen
-------
Este documento recoge las asunciones actuales que hace el backend textual LLVM (`LlvmTextBackend`) sobre la ABI y las convenciones de llamada. Su propósito es
- dejar explícitas las decisiones actuales para lowering y runtime,
- servir como referencia para pruebas y para futuras integraciones con bindings LLVM (por ejemplo `inkwell`).

Defaults del backend
--------------------
- Target triple por defecto: `x86_64-unknown-linux-gnu` (ver `CodegenContext::DEFAULT_TARGET_TRIPLE`).
- Data layout por defecto: la cadena en `CodegenContext::DEFAULT_DATA_LAYOUT`.
- Convención de llamadas por defecto: la convención C-like por defecto del target (no se emite explícita `callingconv` salvo que se requiera más adelante).

Mapeo de tipos (resumido)
------------------------
- `Number` / `double` -> `double` (IEEE 754 double)
- `Boolean` -> `i1`
- `String` / objetos manejados -> `ptr` (puntero opaque en el IR textual)
- enteros (general) -> `i64` (se usa `i64` para tamaños/offsets/índices por simplicidad)
- tipos específicos `i8/i16/i32/i64` se preservan si se usan explícitamente

Representación y paso de parámetros
----------------------------------
- Valores escalares (floating/integer/boolean) se representan con sus tipos LLVM (`double`, `i64`, `i1`) y se pasan directamente.
- Strings y valores heap-allocated se representan como `ptr` y se pasan como `ptr` (paso por valor de puntero).
- Estructuras complejas y arreglos: por ahora no hay un esquema avanzado — se pasarían como `ptr` a objetos gestionados o se ampliará la estrategia cuando se implementen structs concretos.

Retorno de valores
------------------
- El retorno de funciones sigue el tipo mapeado: `double` para números, `i64`/`i1` para enteros/booleanos, `ptr` para referencias. El backend textual emite `define <ret> @name(...)` con el tipo calculado.

Runtime y funciones externas
---------------------------
- El backend registra y declara helpers runtime por defecto:
  - `declare void @print(ptr)`
  - `declare ptr @hulk_alloc(i64)`
  - `declare void @hulk_free(ptr)`
  - `declare i64 @hulk_strlen(ptr)`
- Estas declaraciones deben coincidir con la implementación real del runtime; cambiarlas requiere actualizar `LlvmModule::add_default_runtime_decls()`.

Orden de emisión y símbolos
--------------------------
- El backend emite en orden: header (ModuleID, target triple, datalayout) → `declare` runtime → `declare` prototipos externos → `define` funciones con cuerpo.
- Si una función aparece primero como `declare` y luego se define, el backend evita emitir duplicados: la declaración inicial se marca como definida y no se re-emite `declare` redundante.

Limitaciones actuales y supuestos
--------------------------------
- No hay implementación completa de convenciones de llamada específicas del ABI (p. ej. llamadas en registros x86_64 vs montones). El backend textual emite tipos y confiará en el verificador/target or toolchain para comprobaciones más finas.
- Asumimos arquitectura de 64-bit (por el uso de `i64` para tamaños/offsets). Para targets de 32-bit habrá que ajustar `CodegenContext::DEFAULT_TARGET_TRIPLE` y las reglas de mapeo.

Validación y pruebas
--------------------
- El pipeline realiza checks básicos: conteo de argumentos en llamadas vs firma registrada, símbolos desconocidos y conflictos de firma. Estos checks se ejecutan en `LlvmLifecycle::validate_full_module()` antes de devolver la IR.
- Recomendación: habilitar verificación con LLVM real usando la feature `llvm-verify` (requiere instalar LLVM y activar `inkwell`) para comprobar invariantes más estrictas.

Cómo cambiar la convención o las reglas
--------------------------------------
- Para cambiar la convención por defecto (por ejemplo usar `fastcc`), actualizar:
  - `LlvmTextBackend::render_function()` para emitir `define <ret> @<name>(...) #<attr>` o `callingconv` cuando corresponda.
  - Centralizar la elección de `callingconv` en `CodegenContext` o una nueva estructura `AbiConfig` para permitir configuraciones por módulo/función.
- Para soportar passing/return de structs por val: añadir reglas en `llvm_type_for` y en la fase de lowering para descomposición o pointer-elision.

Ejemplos
--------
- Emisión típica de función definida:

  define i64 @main() {
    entry:
      ret i64 0
  }

- Emisión de helper runtime:

  declare void @print(ptr)

Notas finales
------------
Este documento resume las decisiones actuales y está pensado como un punto de partida. A medida que se integre un backend nativo (inkwell) y se necesiten optimizaciones/compatibilidades con toolchains específicas, las reglas aquí descritas deberán revisarse y ampliarse.

TODO: añadir una sección con la lista completa de atributos (`nocapture`, `readonly`, `nonnull`) y cuándo aplicarlos para optimizaciones.
