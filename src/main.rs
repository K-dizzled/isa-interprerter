use clap::{arg, command, Command};

fn main() {
    let matches = command!()
        .about("An interpreter for simple ISA with shared weak memory")
        .version("1.0")
        .author("Andrei Kozyrev")
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .about("Run an interpreter on a given program")
                .arg(arg!([MEMORY_MODEL] "Which memory model to use: SC, TSO or PSO.").short('m').required(true))
                .arg(arg!([PROGRAM_PATHS] "List of paths to programs to run in different threads. Format: \'<path1>, <path2>, ...\'").short('p').required(true))
        )
        .get_matches();

    match matches.subcommand() {
        Some(("run", sub_matches)) => {
            let memory_model = sub_matches.get_one::<String>("MEMORY_MODEL").unwrap();
            let program_paths = sub_matches
                .get_one::<String>("PROGRAM_PATHS")
                .unwrap()
                .split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<String>>();

            match memory_model.as_str() {
                "SC" => {
                    let mut inter = isa_interpreter::InterpretorSC::new(program_paths);
                    inter.run();
                }
                "TSO" => {
                    let mut inter = isa_interpreter::InterpretorTSO::new(program_paths, false);
                    inter.run();
                }
                "PSO" => {
                    let mut inter = isa_interpreter::InterpretorTSO::new(program_paths, true);
                    inter.run();
                }
                _ => panic!("Invalid memory model"),
            }
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }
}
