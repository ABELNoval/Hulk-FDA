# HULK Compiler Interface Contract

All submissions must implement this interface exactly for automated grading.

## Build

```bash
make build
```

- Must compile the project from source on Ubuntu (latest LTS).
- Must produce a `./hulk` executable **in the repo root**.

## Invoke

```bash
./hulk <file.hulk>
```

**On success (exit 0):**
- Produces `./output` executable in the current directory (Linux x86_64).

**On error:**

Exits with the code corresponding to the error type:

| Code | Type |
|------|------|
| 1 | `LEXICAL` |
| 2 | `SYNTACTIC` |
| 3 | `SEMANTIC` |

Prints **one line per error** to **stderr**:
[text](<d:/Escuela/Tercer Ano/Segundo Semestre/Asignaturas/Compilacion/Compilator Test/compilers/tests>)
```
(line,col) TYPE: message
```

- `line`, `col`: 1-based position of the first token attributable to the error.
- Use `(0,0)` if there is no sensible position.
- `TYPE`: exactly `LEXICAL`, `SYNTACTIC`, or `SEMANTIC`.
- If errors of multiple types exist, exit code reflects the most fundamental type
  (LEXICAL takes priority over SYNTACTIC, SYNTACTIC over SEMANTIC).
