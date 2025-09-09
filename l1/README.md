# brilro
This is a tool which rotates bril functions. It also builds basic blocks and CFGs!

## Dependencies
Nothing specific. Simply common OS ones you probably already have. Though a unix shell will make testing much easier.

## Installation
```
cargo install --path .
```

## Testing
Testing this tool depends on [bril2json](https://github.com/jku20/bril) and [turnt](https://github.com/cucapra/turnt) as well as normal unix tools like `make`.

To run tests, run
```bash
make test
```
