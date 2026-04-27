mod cli;
mod pipeline;
mod lexer;
mod parser;
mod semantic;
mod ir;
mod codegen;
mod utils;

// Cambia estas dos líneas
#[cfg(test)]
#[path = "lexer/tests.rs"] // <--- Esto le indica la ruta exacta relativa a src/
mod tests;

fn main() {
    println!("Hulk Compiler initialized.");
}
