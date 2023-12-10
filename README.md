# ISA Weak Memory Interpreter
An interpreter for simple ISA with shared weak memory. 

## 🔨 Build and run
```sh
cargo build
./target/debug/isa_interpreter run -m ${MEMORY_MODEL} -p ${PROGRAMS}
```

## 📋 Parameters 
```sh
$ ./target/debug/isa_interpreter run --help
Run an interpreter on a given program

Usage: isa_interpreter run -m <MEMORY_MODEL> -p <PROGRAM_PATHS>

Options:
  -m <MEMORY_MODEL>       Which memory model to use: SC, TSO or PSO.
  -p <PROGRAM_PATHS>      List of paths to programs to run in different threads. Format: '<path1>, <path2>, ...'
  -h, --help              Print help
  -V, --version           Print version
```

## 📜 Usage
When you run a `run` command, the interpreter will run the given programs in different threads. The programs are run in the order they are given. The interpreter will ask for your choice of the next executed line at each step. 

Apart from choosing the next line you can use one of the following commands:
- `exit` Exit the interpreter.
- `memory` Print the current state of the memory.
- `registers` Print the current state of the registers.
- `graph <path>` Save the current execution graph to a file at the given path. The file will be saved in the `dot` format. You can use [Graphviz](https://graphviz.org/) to visualize the graph, or, if you have `dot` installed, you can use the `dot` command to convert the file to a different format. For example, to convert the file to a `png` image, you can run: 
```sh
dot -Tpng <dot-file-path> -o <png-file-path>
```

## 📚 Examples



