# brilro
This is a tool which rotates bril functions. It also builds basic blocks and CFGs!

## Dependencies
brilro depends on [brili](https://github.com/jku20/bril) to run bril programs.

## Installation
```
cargo install --path .
```

## Usage
```
brilro --help
```

## Testing
Testing this tool depends on [bril2json](https://github.com/jku20/bril), [bril2txt](https://github.com/jku20/bril), and [turnt](https://github.com/cucapra/turnt) as well as normal unix tools like `make`.

To run tests, run
```bash
make test
```
