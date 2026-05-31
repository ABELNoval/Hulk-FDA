// =============================================================================
// Tests Unitarios - CLI
// =============================================================================
//
// Tests para el módulo CLI. Aquí se prueban:
//
// - Parsing correcto de argumentos
// - Manejo de flags inválidos
// - Valores por defecto
// - Combinaciones de opciones
// - Mensajes de error apropiados
//
// =============================================================================

#[cfg(test)]
mod tests_cli {
    use crate::{CliCommand, CompilationMode, InputSource};

    #[test]
    fn parses_help_flag() {
        let command = CliCommand::parse_args(vec!["--help".to_string()]).unwrap();
        assert!(matches!(command, CliCommand::Help));
    }

    #[test]
    fn parses_file_and_mode() {
        let command =
            CliCommand::parse_args(vec!["--parse".to_string(), "programa.hulk".to_string()])
                .unwrap();

        match command {
            CliCommand::Run(config) => {
                assert_eq!(config.mode, CompilationMode::Parse);
                assert!(matches!(config.input, InputSource::File(_)));
            }
            CliCommand::Help => panic!("se esperaba una corrida, no ayuda"),
        }
    }

    #[test]
    fn parses_inline_source() {
        let command =
            CliCommand::parse_args(vec!["--source".to_string(), "1 + 2".to_string()]).unwrap();

        match command {
            CliCommand::Run(config) => {
                assert_eq!(config.mode, CompilationMode::Semantic);
                assert!(matches!(config.input, InputSource::Inline(_)));
            }
            CliCommand::Help => panic!("se esperaba una corrida, no ayuda"),
        }
    }

    #[test]
    fn parses_ir_mode() {
        let command =
            CliCommand::parse_args(vec!["--ir".to_string(), "programa.hulk".to_string()]).unwrap();

        match command {
            CliCommand::Run(config) => {
                assert_eq!(config.mode, CompilationMode::Ir);
                assert!(matches!(config.input, InputSource::File(_)));
            }
            CliCommand::Help => panic!("se esperaba una corrida, no ayuda"),
        }
    }

    #[test]
    fn parses_codegen_mode_and_output() {
        let command = CliCommand::parse_args(vec![
            "--codegen".to_string(),
            "-o".to_string(),
            "out.ll".to_string(),
            "programa.hulk".to_string(),
        ])
        .unwrap();

        match command {
            CliCommand::Run(config) => {
                assert_eq!(config.mode, CompilationMode::Codegen);
                assert_eq!(
                    config.output.as_deref(),
                    Some(std::path::Path::new("out.ll"))
                );
            }
            CliCommand::Help => panic!("se esperaba una corrida, no ayuda"),
        }
    }
}
