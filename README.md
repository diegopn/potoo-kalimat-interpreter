<p align="center">
  <img src="https://diegopn.github.io/potoo-site/src/assets/images/language-interpreter-icon.svg?v=1c651ca7" alt="Potoo IDE language interpreter icon" width="120" />
</p>

<h1 align="center">Kalimat Interpreter</h1>

<p align="center">A host-driven Rust interpreter for the documented Kalimat runtime.</p>

<p align="center">
  <a href="https://github.com/diegopn/potoo-kalimat-interpreter/releases">Download</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/kalimat/">Website</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/kalimat/">User manual</a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"></a>
  <a href="https://doc.rust-lang.org/cargo/"><img alt="Cargo" src="https://img.shields.io/badge/Cargo-DEA584?style=for-the-badge&logo=rust&logoColor=white"></a>
  <img alt="Rust library" src="https://img.shields.io/badge/Rust_library-4A86CF?style=for-the-badge">
  <img alt="CLI" src="https://img.shields.io/badge/CLI-6A4C93?style=for-the-badge">
</p>

This Rust package is part of the Potoo IDE ecosystem and is separate from the desktop interface. Compatibility with the IDE has not been implemented or verified.

## Reference baseline

The implementation targets the Arabic syntax and runtime features shown in the official distribution's bundled reference and examples guide PDFs. Those documents are legacy references: the deck shows dates from 2010–2011 and does not define every detail of the current distribution. Where the references are silent, this crate avoids claiming compatibility.

The documented baseline covers dynamic values, input and output, conditions and loops, arrays and dictionaries, procedures and functions, built-ins, tail calls, drawing and sprites, objects and methods, events, cooperative processes and CSP channels, files, and units. See [reference coverage](REFERENCE-COVERAGE.md) for implemented behavior and host responsibilities. This crate does not reproduce the Kalimat IDE.

## Current limits

The reference PDFs do not specify C FFI syntax or behavior. The official site advertises FFI, and a 2013 creator article describes it, but its status in the 2025 distribution is unclear. This crate does not implement or claim FFI compatibility.

The PDFs explicitly say that `do/while`, `break`, and `continue` are unavailable, custom data types are not yet supported as dictionary keys, constructors are absent, and process launch is asynchronous without true parallel execution. These limits are preserved in the documented profile.

## Status

This project remains under evaluation. Current Arabic compatibility has not been established, and no downloadable binaries are available yet.

## Library use

Call `parse(source)`, then run the resulting `Program` with `Interpreter::new(host).run(&program)`. `Host` supplies random values and routes graphics, sprites, GUI calls, event registration, channels, file operations, and asynchronous launch. Its default input and output methods use the terminal.

## Command line

```sh
cargo run --locked -- source.kalimat
```

The runner reads the UTF-8 source file path from its command-line argument and resolves unit files beside the source file. It provides terminal input/output, random values, and delays. Graphical, file, event, and concurrent host operations need an application-specific host.

## References

- [Official Kalimat site](https://kalimat-lang.com/)
- [Official getting-started guide](https://kalimat-lang.com/getting-started.html), which describes the grammar PDFs included with the distribution.
- [Creator's March 2013 article](https://iamsamy.blogspot.com/2013/03/kalimat-educational-programming.html), a historical source for additional features whose current status is not established by the bundled PDFs.

## Support

If this project is useful to you, you can support its continued development:

<p align="left">
  <a href="https://github.com/sponsors/diegopn"><img alt="Become a sponsor" src="https://img.shields.io/badge/Become_a_sponsor-EA4AAA?style=for-the-badge&logo=githubsponsors&logoColor=white"></a>
</p>

## Project policies

- [Code of conduct](CODE_OF_CONDUCT.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [GPL-3.0-or-later license](LICENSE)
