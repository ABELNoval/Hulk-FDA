# HULK Compiler — Informe técnico

## 1. Introducción

Este documento describe la arquitectura, las decisiones de diseño y el estado
funcional del compilador HULK implementado en este repositorio. El objetivo
fue construir un compilador de extremo a extremo para el lenguaje HULK
(**Havana University Language for Kompilers**), organizado en fases bien
separadas: análisis léxico, análisis sintáctico, análisis semántico,
generación de representación intermedia (BANNER IR), generación de LLVM IR
y ejecución del binario mediante una cadena nativa de compilación.

El proyecto no se limitó a una traducción superficial del código fuente.
Desde el inicio se buscó una estructura que permitiera depuración por etapas,
diagnósticos precisos y extensibilidad. Adicionalmente se implementaron dos
extensiones al lenguaje base — **Protocolos** e **Iterables con generadores** —
que atraviesan todas las fases del compilador y son el foco principal de este
informe.

La implementación está escrita en **Rust** y se organiza en módulos
independientes por etapa. Hay pruebas por capas: léxico, parser, semántica,
IR, codegen y ejecución, en lugar de depender únicamente de pruebas end-to-end.

---

## 2. Estructura general del proyecto
src/

├── lexer/       análisis léxico y tokenización

├── parser/      construcción del AST

├── semantic/    verificación semántica y sistema de tipos

├── ir/          BANNER IR, lowering y CFG

├── codegen/     emisión de LLVM IR

├── runtime/     biblioteca nativa con ABI C (libruntime_rs.a)

├── pipeline/    orquestación de fases

├── execution/   automatización del flujo nativo

└── cli/         interfaz de línea de comandos

La raíz de la biblioteca expone los tipos principales desde `lib.rs`, lo que
permite que los tests de integración y el binario principal reutilicen los
mismos componentes sin acoplarse a detalles internos.

---

## 3. Análisis léxico

El lexer está implementado manualmente en Rust como un **Autómata Finito
Determinista (AFD) explícito**. Recorre el fuente secuencialmente produciendo
tokens acompañados de información de localización (*span*: archivo, línea,
columna inicial y final) que se propaga a todas las fases posteriores.

El diseño prioriza dos cosas: precisión del token y trazabilidad del error.
Cuando el lexer no puede construir un token válido — carácter inesperado,
cadena sin cerrar, secuencia de escape inválida — registra un diagnóstico
acumulado. El pipeline toma el primer error relevante y lo eleva como
`CompilationError`, deteniendo las fases siguientes.

Las extensiones de Protocolos e Iterables incorporaron tokens nuevos que
coexisten con el vocabulario base: `protocol` y `extends` para la declaración
de protocolos; `is` y `as` para verificación y conversión de tipos; `->` para
anotar tipos función; `||` como separador en comprensiones de vector; y `*`
en posición de sufijo de tipo para denotar iterables (`Number*`). La
distinción de operadores compuestos (`==`, `!=`, `<=`, `:=`, `=>`, `->`,
`@@`, `||`) se maneja mediante *lookahead* de un carácter.

---

## 4. Análisis sintáctico y AST

El parser está implementado manualmente con la técnica de **Recursive Descent
Parsing**, apoyado en un cursor de tokens que permite avanzar, mirar adelante
y hacer retrocesos controlados. Cada producción de la gramática corresponde a
una función que construye el nodo AST correspondiente.

El AST modela HULK como un **lenguaje orientado a expresiones**: construcciones
como `if`, `while`, `let` y bloques producen valores y se tratan con la misma
jerarquía que cualquier expresión. Esto simplifica el análisis semántico y
el lowering, porque la mayoría de las reglas se pueden describir en términos
de un único tipo de nodo.

Un aspecto de diseño relevante es el nodo `TypeReference`, que soporta cuatro
variantes:

| Variante | Sintaxis | Uso |
|---|---|---|
| `Named(String)` | `Number`, `MyType` | Tipos nominales |
| `Iterable(T)` | `Number*` | Parámetros iterables |
| `Vector(T)` | `Number[]` | Tipos vector |
| `Function(params, T)` | `(Number) -> Boolean` | Tipos función |

Modelar estas variantes directamente en `TypeReference` permite que el
analizador semántico las valide de forma uniforme sin casos especiales.
Las extensiones añadieron los nodos `ProtocolDeclaration`, `For`, `VectorLiteral`,
`VectorComprehension`, `Lambda`, `TypeCheck` y `TypeCast`.

---

## 5. Análisis semántico

La etapa semántica verifica que el programa tenga sentido conforme a las
reglas del lenguaje. Su arquitectura se divide en cuatro componentes que
colaboran mediante un contexto compartido: **Semantic Context**, **Symbol
Table**, **Type Environment** y **Expression Checker**.

### Tipo normalizado

El elemento central del diseño es el enum `NormalizedType`, que representa
internamente todos los tipos independientemente de la sintaxis del fuente:

```rust
enum NormalizedType {
    Unknown, Number, Boolean, String, Void,
    Named(String),
    Protocol(String),
    Iterable(Box<NormalizedType>),
    Vector(Box<NormalizedType>),
    Function(Vec<NormalizedType>, Box<NormalizedType>),
}
```

Separar la sintaxis del fuente (`TypeReference`) de la semántica interna
(`NormalizedType`) permite que los algoritmos de compatibilidad e inferencia
operen sobre una representación canónica.

### Tabla de símbolos

La tabla de símbolos es jerárquica (*stack of scopes*). Registra variables,
parámetros, funciones, tipos y protocolos. Las búsquedas proceden del ámbito
más interno al global, implementando *shadowing*. Los errores se acumulan en
el contexto en lugar de abortar al primer fallo, lo que permite reportar
múltiples problemas en una sola ejecución.

### Protocolos — verificación estructural

Un tipo *T* conforma al protocolo *P* si implementa todos los métodos
requeridos con firmas compatibles. La verificación es **estructural**: no se
necesita declarar `implements P`. Si *P* extiende a *Q*, los métodos de *Q*
se exigen también en *P*. La conformidad por herencia transitiva es suficiente;
las subclases no necesitan re-declarar la conformidad.

### Iterables — duck typing estático

Un tipo es iterable si posee los métodos `next(): Boolean` y `current(): T`.
El tipo anotado `T*` acepta cualquier tipo que satisfaga esta condición. El
builtin `range(lo, hi)` se registra con firma `(Number, Number) → Number*`.
La variable de iteración en un `for` recibe tipo `T` inferido desde el
iterable.

---

## 6. Representación intermedia — BANNER IR

Una vez validado el AST se genera **BANNER IR**, una representación intermedia
propia inspirada en LLVM IR. Sus propiedades clave son: forma **SSA** (*Static
Single Assignment*), bloques básicos con un único punto de entrada y salida,
**CFG** explícito con predecesores y sucesores por bloque, instrucciones de
tres direcciones y soporte para llamadas indirectas vía vtable (`CallIndirect`).

La jerarquía es: `IRModule` → `IRFunction` → `BasicBlock` → `IRInstruction`
→ `IROperand/IRValue`. Los identificadores SSA siguen la convención `%t0`,
`%t1`, … para temporales y `bb0`, `bb1`, … para bloques.

### Lowering

El lowering transforma el AST tipado en BANNER IR mediante un recorrido
recursivo descendente. El contexto de lowering mantiene el módulo en
construcción, la función y bloque activos, el generador de nombres SSA y el
entorno variables→valores SSA. El **CFG se construye incrementalmente**: al
aparecer un `if`, `while` o `for`, los bloques necesarios y sus aristas se
crean de inmediato sin análisis posterior.

El `for` loop es la construcción más relevante de la extensión de Iterables.
Su traducción genera tres bloques básicos y despacha dinámicamente por vtable:
primero evalúa el iterable (`%iter`), luego llama `CallIndirect next(%iter)`,
y en el bloque del cuerpo llama `CallIndirect current(%iter)`. Para el builtin
`range`, se emite `range(lo, hi, @__vtable_Range)` pasando la vtable como
parámetro de forma que el dispatch dinámico funcione igual que con tipos
definidos por el usuario.

### Vtables

Cada tipo con métodos y cada protocolo generan un global de punteros a función:

```llvm
@__vtable_Square = global [2 x ptr] [
    ptr @Square_area,
    ptr @Square_name
]
```

El despacho sigue cuatro pasos: `Load` del puntero de vtable (offset 0 del
objeto), `GetElementPtr` al slot del método, `Load` del puntero a función, y
`CallIndirect`. Este mecanismo es idéntico para protocolos e iterables.

### SSA y CFG

Introducir SSA en la IR propia fue una decisión clave. La mayoría de los
errores difíciles de depurar en el backend aparecen cuando se violan las
expectativas de dominancia o cuando un phi node no recibe entradas válidas.
Al forzar estas reglas en BANNER IR se detectan problemas antes de emitir LLVM
inválido. El CFG implementa recorridos DFS, BFS y PostOrder (necesarios para
construcción de dominadores y renombrado SSA) y valida internamente la
consistencia de aristas y la alcanzabilidad de bloques.

---

## 7. Generación de código LLVM

El backend se organiza alrededor del trait `CodegenBackend` con dos
implementaciones:

- **`LlvmTextBackend`**: emite LLVM IR textual (`.ll`) construyendo cadenas.
  No requiere que las bibliotecas de desarrollo de LLVM estén presentes en
  tiempo de compilación del compilador; delega el procesamiento a herramientas
  externas (`opt`, `llc`, `clang`).

- **`LlvmInkwellBackend`**: usa los bindings `inkwell` para la C API de LLVM
  (feature `llvm-verify`). Requiere `libLLVM` en tiempo de compilación y
  permite verificación formal del módulo IR en-proceso, detectando IR
  malformado con mejores mensajes antes de pasarlo a las herramientas externas.

El backend emite el módulo en orden: cabecera (target triple, datalayout),
vtable globales, declaraciones de runtime, prototipos externos y definiciones
de funciones. El mapeo de tipos es: `Number` → `double`, `Boolean` → `i1`,
`String` y objetos → `ptr`, tamaños e índices → `i64`. El target por defecto
es `x86_64-unknown-linux-gnu`.

Un problema resuelto durante el desarrollo fue el manejo de **phi nodes**: los
valores entrantes deben emitirse antes del terminador del bloque predecesor.
Colocar una instrucción en posición inválida hace que el verificador LLVM
rechace el módulo. Otro punto fue la materialización de **literales de texto**
como cadenas globales con punteros válidos al runtime, no como valores nulos.

---

## 8. Runtime y ABI

El proyecto incluye un runtime en Rust (`libruntime_rs.a`) que expone
funciones C-compatible (`#[no_mangle] pub extern "C" fn`). Esta separación
permite que el runtime evolucione independientemente del compilador y que el
backend LLVM asuma una interfaz estable.

Las categorías principales son:

| Categoría | Funciones representativas |
|---|---|
| Memoria | `hulk_alloc(i64)→ptr`, `hulk_free(ptr)` |
| Cadenas | `hulk_concat`, `hulk_concat_space`, `hulk_num_to_str` |
| Impresión | `print_number`, `print_string`, `print_bool` |
| Iterables | `range(double,double,ptr)→ptr`, `Range_next`, `Range_current` |
| Vectores | `hulk_vector_new`, `hulk_vector_push`, `hulk_vector_get` |
| Matemáticas | `pow(double,double)→double`, `strcmp(ptr,ptr)→i32` |

El tipo `Range` en el runtime tiene un puntero de vtable en el offset 0 para
que el dispatch dinámico del `for` loop funcione de manera idéntica al de
cualquier tipo definido por el usuario.

---

## 9. Pipeline y ejecución nativa

El módulo `pipeline` orquesta las fases en orden y expone modos de parada
intermedios (`--lex`, `--parse`, `--semantic`, `--ir`, `--codegen`, `--run`).
Esto permite inspeccionar el compilador por etapas y escribir tests que
verifican el estado intermedio, no solo el éxito o fracaso final.

Una vez emitido el `.ll`, el flujo nativo sigue:

.ll  →[opt -O2]→  .bc  →[llc -relocation-model=pic]→  .o

↓

[clang -fPIE + libruntime_rs.a]→  ejecutable

La capa `execution` encapsula esta orquestación resolviendo rutas y armando
comandos, evitando que la lógica de integración quede dispersa entre `main`
y scripts externos.

---

## 10. Extensiones implementadas

### Protocolos

Los protocolos permiten definir contratos de comportamiento que cualquier
tipo puede satisfacer sin herencia explícita:

```hulk
protocol Shape {
    area(): Number;
    name(): String;
}

protocol Drawable extends Shape {
    draw(): String;
}
```

Un tipo satisface el protocolo simplemente implementando los métodos
requeridos. No existe una declaración `implements`. En el backend, los métodos
de protocolo se despachan por vtable exactamente igual que los métodos
virtuales de tipos ordinarios.

### Iterables y generadores

Cualquier tipo con `next(): Boolean` y `current(): T` es un iterable de `T`.
No hace falta declararlo:

```hulk
type OddNumbers(limit: Number) {
    current_val: Number = -1;
    next(): Boolean {
        self.current_val := self.current_val + 2;
        self.current_val <= limit;
    }
    current(): Number { self.current_val; }
}

for (x in new OddNumbers(9)) { print(x); }  // 1 3 5 7 9

// Función genérica sobre cualquier iterable de Number
function sum_gen(gen: Number*): Number {
    let s: Number = 0 in {
        for (x in gen) { s := s + x; };
        s;
    };
}
```

El `for` siempre emite `CallIndirect` sobre la vtable, de modo que el builtin
`range` y los generadores de usuario usan exactamente el mismo mecanismo.

---

## 11. Estrategia de errores

El proyecto define una jerarquía unificada de errores que cubre léxico,
sintaxis, semántica, I/O e internos del compilador. Cada error conserva la
fase en que se originó y la posición exacta en el fuente. Los errores
semánticos se acumulan en el contexto en lugar de abortar al primer fallo,
permitiendo reportar múltiples problemas en una sola ejecución. Esta política
es importante tanto para la experiencia del usuario como para la
compatibilidad con sistemas de evaluación automatizada.

---

## 12. Limitaciones y trabajo futuro

Las extensiones de **Vectores** y **Functores** tienen soporte en el parser
y el sistema de tipos, pero su integración completa en el lowering y el
runtime está pendiente. Para Vectores, resta el manejo de vectores como
valores de primera clase en todas las posiciones del lenguaje. Para Functores,
resta el lowering de cierres con captura de entorno y el dispatch de llamadas
a functores como `CallIndirect`.

Otras líneas abiertas: optimizaciones sobre BANNER IR (propagación de
constantes, eliminación de código muerto, devirtualización cuando el tipo
concreto es conocido en el sitio de llamada), extensión de la inferencia de
tipos a un esquema bidireccional, emisión de metadatos DWARF para depuración
con `gdb`/`lldb`, y una biblioteca estándar escrita en HULK sobre las
extensiones de Iterables y Vectores.

---

## 13. Conclusión

El compilador HULK está estructurado como una tubería completa y modular.
Lexer, parser, semántica, BANNER IR y backend no son piezas aisladas: forman
un sistema coherente donde cada fase prepara el terreno para la siguiente.
La decisión de verificación estructural para protocolos e iterables resultó
coherente con la filosofía del lenguaje: el compilador valida contratos de
comportamiento, no linajes de herencia, y el mismo mecanismo de vtable sirve
para ambas extensiones sin duplicación en el backend. El runtime en Rust con
ABI C garantiza seguridad de memoria en operaciones críticas sin introducir
un recolector de basura. El siguiente paso es completar las extensiones de
Vectores y Functores y alinear la interfaz de entrega con el contrato de
evaluación.
