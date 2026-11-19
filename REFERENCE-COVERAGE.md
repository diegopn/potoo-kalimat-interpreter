# Kalimat reference coverage

This document compares the crate's current source-level implementation with the 2025 distribution's bundled `مرجع كلمات.pdf` reference deck and `تعلم كلمات بالأمثلة.pdf` by-example guide. The deck is a legacy baseline, with slides dated 2010–2011; it does not establish full compatibility with the 2025 release. The page references below are PDF page numbers. The distribution is available through the [official Kalimat site](https://kalimat-lang.com/) and described by its [getting-started guide](https://kalimat-lang.com/getting-started.html).

The table records code coverage, not behavioral validation. The allowed verification for this implementation was formatting, locked compilation, and whitespace-diff checking; no Kalimat program was run.

| Feature | Reference | Crate coverage |
| --- | --- | --- |
| Dynamic values and no declarations | Deck pp. 4–5, 20–21 | Numbers, strings, booleans, arrays, dictionaries, objects, classes, channels, and host file handles are represented dynamically. Locals are created by assignment or parameters; `shared` exposes main-program variables to procedures. |
| Input and output | Deck pp. 6–7; guide p. 1 | `read` supports an optional quoted prompt, multiple destinations, and `#` numeric destinations. `print` accepts multiple expressions; a trailing `...` suppresses the newline. Default host I/O is terminal-based; hosts can supply their own input and output behavior. |
| Boolean logic, comparisons, branches | Deck pp. 8–10 | Boolean conditions, comparisons, `and`/`or`/`not`, and `if`/`else if`/`else` are parsed. `and` and `or` short-circuit. |
| Loops and jumps | Deck pp. 11–13; guide p. 1 | `while`, infinite loops, inclusive ascending/descending counted loops with explicit steps, and labels/`goto` are implemented. A jump resolves within its containing statement block and propagates outward until a matching label is found. |
| Arrays | Deck pp. 14–16; guide p. 2 | Mixed-value arrays, array literals, dimensions, 1-based indexing, and reference assignment are implemented. Multi-index access is evaluated one dimension at a time. |
| Dictionaries | Deck p. 17 | Dictionary literals and indexed read/write are implemented. Keys are integers, strings, or arrays composed of those key types; array keys compare by value. Custom data types as keys are not supported, matching the manual's stated limitation. |
| Procedures, functions, and scope | Deck pp. 18–21; guide p. 1 | Procedure and function definitions/calls, explicit return values, local variables, and main-program `shared` variables are implemented. Function completion without a value is an error. |
| Built-ins | Deck pp. 22–23; guide pp. 2–3 | The documented math, string, conversion, type, array, and random functions are implemented except for ambiguous `حرف`, which is delegated to the host. `constant` implements the documented newline token and delegates other constant names. String slicing counts Unicode scalar values; `mid` uses the guide's zero-based start. The references do not specify trigonometric angle units; this implementation uses Rust's radian-based math functions. |
| Tail calls | Deck p. 24 | `delegate to` calls are handled by a trampoline so direct delegated recursion does not consume one Rust call frame per iteration. |
| Drawing and sprites | Deck pp. 25–29; guide p. 2 | Documented drawing forms and sprite built-ins are routed through `Host`. A host must implement the graphics or image operations it supports; the command-line host has no graphical surface. |
| Classes, fields, methods, inheritance | Deck pp. 30–40; guide pp. 2, 4 | Classes, fields, object references, explicit method receivers, declared/implemented methods, base-class lookup, and `is` type checks are implemented. Constructors are absent in the reference, so instances begin with uninitialized fields. |
| Events and keyboard input | Deck pp. 41–43 | Event declarations are sent to the host. `Interpreter::dispatch_event` invokes the registered procedure with host-supplied arguments. Event delivery, key translation, and mouse payloads remain host responsibilities. |
| GUI | Deck pp. 48–57 | Widget creation and method calls are routed through the generic host callback. No GUI toolkit, event loop, or visual environment is implemented in the crate. |
| Cooperative launch and CSP channels | Deck pp. 44–47 | Launch, channel creation, send/receive, and `select` are represented as host operations. The host owns scheduling and rendezvous semantics; the crate does not provide a parallel scheduler. |
| Files | Guide pp. 5–6 | File creation/open/append calls, file methods, and `print in`/`read from` operations route through host callbacks. A host must provide the file-handle behavior. |
| Units | Guide p. 6 | `using` loads a file through `Host::load_unit`; unit definitions and simple global assignments are merged before the main program runs. Unit names remain unused, as the guide says. The command-line loader resolves files beside the main source file. |

## Documented omissions and unresolved claims

- The deck says there is no `switch` equivalent (p. 10), `do/while` is planned but unavailable (p. 11), and `break`/`continue` are deliberately absent (p. 13). This crate does not add those forms.
- The deck says custom data types as dictionary keys are planned (p. 17), constructors are absent (p. 40), and process launch is asynchronous but not true parallel execution yet (p. 44). Those remain unsupported or host-dependent.
- The guide says unit names are currently unused and may later support namespacing (p. 6); this implementation does not namespace unit definitions.
- C FFI is advertised by the official site and discussed in a 2013 creator article, but it is absent from the bundled PDFs and its 2025 release status is unclear. This crate does not implement or claim FFI compatibility.
- Lambda, destructuring, and parser-generator features are not defined by the bundled PDFs. They are outside this baseline and are not claimed.

The old creator article is retained only as historical context in the README. It is not treated as authority for current release behavior.
