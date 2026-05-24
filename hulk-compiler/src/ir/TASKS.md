# Shared Workflow for SSA / CFG / IR

Este documento define el sistema de tareas compartidas para el hito de SSA, CFG e IR. La idea es que los tres frentes trabajen sobre una base común, con interfaces, nombres y pruebas compatibles, para evitar solapamientos y conflictos.

## Objetivo general

Mantener un flujo de desarrollo compartido donde:

- la IR tenga contratos públicos claros,
- el CFG use las mismas convenciones que SSA,
- las pruebas sean reutilizables,
- cada cambio nuevo llegue con validación enfocada antes de integrarse.

## Tareas

### 1. Definir las interfaces públicas que usarán los otros tres desarrolladores

Esto significa fijar qué tipos, funciones y estructuras son estables y pueden consumirse desde otros módulos sin romper compatibilidad.

Incluye:

- qué representa un módulo IR,
- cómo se crean y leen bloques básicos,
- cómo se modelan valores SSA,
- qué funciones de lowering estarán disponibles,
- qué errores o resultados se devuelven en cada operación.

Meta: que cada equipo trabaje sobre la misma API y no reimplemente conceptos parecidos con nombres distintos.

Estado actual: implementada como una API pública separada en `mod.rs`, `module.rs`, `block.rs`, `instruction.rs`, `value.rs`, `lowering.rs` y `naming.rs`.

### 2. Crear el harness de pruebas compartido para IR, CFG y lowering

Un harness es una base común para ejecutar pruebas con la misma forma de entrada y salida.

Incluye:

- helpers para construir entradas mínimas,
- helpers para ejecutar lowering,
- comparadores de salida esperada,
- utilidades para imprimir o normalizar IR/CFG antes de compararlas.

Meta: poder probar los tres frentes con la misma infraestructura, evitando tests duplicados y frágiles.

Estado actual: implementado como `src/ir/test_support.rs` y validado con tests mínimos en `src/ir/tests.rs`.

### 3. Agregar pruebas basadas en fixtures para entradas AST pequeñas y salidas esperadas

Los fixtures son archivos o datos fijos que representan casos concretos.

Incluye:

- AST pequeños y fáciles de leer,
- IR esperada para cada caso,
- CFG esperada cuando aplique,
- casos de éxito y de error.

Meta: asegurar que casos simples se mantengan correctos y que los cambios no rompan transformaciones básicas.

### 4. Establecer convenciones de nombres para valores SSA, bloques básicos y módulos

La nomenclatura compartida evita confusiones entre los tres implementadores.

Incluye:

- nombres de valores SSA consistentes y legibles,
- nombres de bloques básicos previsibles,
- nombres de módulos estables para pruebas y depuración,
- reglas para temporales, parámetros y resultados intermedios.

Meta: que el output sea fácil de leer, depurar y comparar en tests.

### 5. Revisar integraciones y resolver conflictos entre las tres áreas

Aquí se revisa que SSA, CFG e IR encajen entre sí sin duplicar lógica ni producir estructuras incompatibles.

Incluye:

- revisar cambios cruzados,
- detectar APIs que se pisan,
- corregir diferencias de modelo,
- unificar criterios cuando dos áreas representen lo mismo de forma distinta.

Meta: mantener un solo diseño coherente en lugar de tres versiones paralelas del mismo concepto.

### 6. Asegurar que cada feature nueva tenga al menos un test enfocado antes de mergear

Regla de calidad mínima para cualquier aporte nuevo.

Incluye:

- un test que falle antes del cambio y pase después,
- cobertura del caso principal o del bug corregido,
- validación directa sobre la parte tocada,
- revisión rápida de que no se rompa la ruta feliz.

Meta: evitar merges sin evidencia de funcionamiento y reducir regresiones.

## Criterio de cierre

El bloque SSA/CFG/IR se considera listo cuando:

- existe una API pública estable para el uso compartido,
- los tests comunes cubren lowering, IR y CFG,
- las fixtures representan casos mínimos relevantes,
- la nomenclatura está unificada,
- los conflictos de integración están resueltos,
- todo cambio nuevo llega con al menos un test enfocado.

## Uso práctico

Este archivo sirve como lista de trabajo y como referencia de coordinación. Si quieres, también puedo convertirlo en un checklist con estado `pendiente / en progreso / hecho` para ir marcando avances dentro de `src/ir`.
