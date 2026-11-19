//! A host-driven interpreter for the documented Kalimat reference baseline.
//!
//! The bundled grammar and examples are legacy references, not a formal
//! specification of every current distribution detail. GUI, drawing, file,
//! event, and process effects are delegated to [`Host`].

mod syntax;

pub use syntax::{Diagnostic, Span};

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{self, Write};
use std::rc::Rc;

const MAX_STEPS: u64 = 1_000_000;
const MAX_CALL_DEPTH: usize = 128;

pub type ArrayValue = Rc<RefCell<Vec<Value>>>;
pub type DictionaryValue = Rc<RefCell<Vec<(Value, Value)>>>;
pub type ObjectReference = Rc<RefCell<ObjectValue>>;

#[derive(Clone, Debug)]
pub struct ObjectValue {
    pub class_name: String,
    pub fields: HashMap<String, Value>,
}

/// Dynamically typed values used by the documented runtime.
#[derive(Clone, Debug)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(ArrayValue),
    Dictionary(DictionaryValue),
    Object(ObjectReference),
    Class(String),
    TypeName(String),
    Channel(u64),
    File(u64),
    /// Internal sentinel for declared but not yet assigned values.
    Uninitialized,
}

impl Value {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Boolean(_) => "boolean",
            Self::Array(_) => "array",
            Self::Dictionary(_) => "dictionary",
            Self::Object(_) => "object",
            Self::Class(_) => "class",
            Self::TypeName(_) => "type",
            Self::Channel(_) => "channel",
            Self::File(_) => "file",
            Self::Uninitialized => "uninitialized",
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            _ => None,
        }
    }

    fn display_text(&self) -> String {
        self.display_inner(&mut HashSet::new())
    }

    fn display_inner(&self, seen_arrays: &mut HashSet<usize>) -> String {
        match self {
            Self::Number(number) if number.fract() == 0.0 => format!("{number:.0}"),
            Self::Number(number) => number.to_string(),
            Self::String(value) => value.clone(),
            Self::Boolean(true) => "صحيح".to_owned(),
            Self::Boolean(false) => "خطأ".to_owned(),
            Self::Array(values) => {
                let identity = Rc::as_ptr(values) as usize;
                if !seen_arrays.insert(identity) {
                    return "[<cycle>]".to_owned();
                }
                let text = format!(
                    "[{}]",
                    values
                        .borrow()
                        .iter()
                        .map(|value| value.display_inner(seen_arrays))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                seen_arrays.remove(&identity);
                text
            }
            Self::Dictionary(_) => "{...}".to_owned(),
            Self::Object(object) => format!("<{} object>", object.borrow().class_name),
            Self::Class(name) | Self::TypeName(name) => name.clone(),
            Self::Channel(id) => format!("<channel {id}>"),
            Self::File(id) => format!("<file {id}>"),
            Self::Uninitialized => "<uninitialized>".to_owned(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_text())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => Rc::ptr_eq(a, b),
            (Self::Dictionary(a), Self::Dictionary(b)) => Rc::ptr_eq(a, b),
            (Self::Object(a), Self::Object(b)) => Rc::ptr_eq(a, b),
            (Self::Class(a), Self::Class(b)) | (Self::TypeName(a), Self::TypeName(b)) => a == b,
            (Self::Channel(a), Self::Channel(b)) | (Self::File(a), Self::File(b)) => a == b,
            (Self::Uninitialized, Self::Uninitialized) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeError {
    pub message: String,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Execution {
    pub steps: u64,
}

#[derive(Clone, Debug)]
pub enum ChannelOperation {
    Send {
        channel: Value,
        value: Option<Value>,
    },
    Receive {
        channel: Value,
    },
}

#[derive(Clone, Debug)]
pub struct SelectedOperation {
    pub index: usize,
    pub received: Option<Value>,
}

/// Boundary for input/output and effects owned by a graphical or concurrent host.
/// `host_call` also receives documented operations whose detailed behavior is
/// not specified by the bundled reference, such as `حرف`.
///
/// The old numeric callbacks remain available for simple hosts. Methods added
/// for the documented Arabic runtime have conservative defaults that report
/// an unsupported host operation.
pub trait Host {
    fn random(&mut self, bound: i64) -> Result<i64, String> {
        let _ = bound;
        Err("random source is not provided by the host".to_owned())
    }
    fn draw_line(&mut self, from: (i64, i64), to: (i64, i64), color: i64) -> Result<(), String> {
        self.host_call(
            None,
            "draw_line",
            vec![
                Value::Number(from.0 as f64),
                Value::Number(from.1 as f64),
                Value::Number(to.0 as f64),
                Value::Number(to.1 as f64),
                Value::Number(color as f64),
            ],
        )
        .map(|_| ())
    }
    fn wait(&mut self, milliseconds: i64) -> Result<(), String> {
        let _ = milliseconds;
        Err("wait is not provided by the host".to_owned())
    }
    fn write_output(&mut self, values: &[Value], newline: bool) -> Result<(), String> {
        let mut output = io::stdout().lock();
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                write!(output, " ").map_err(|error| error.to_string())?;
            }
            write!(output, "{value}").map_err(|error| error.to_string())?;
        }
        if newline {
            writeln!(output).map_err(|error| error.to_string())?;
        } else {
            output.flush().map_err(|error| error.to_string())?;
        }
        Ok(())
    }
    fn read_input(&mut self, prompt: Option<&str>, numeric: &[bool]) -> Result<Vec<Value>, String> {
        if let Some(prompt) = prompt {
            print!("{prompt}");
            io::stdout().flush().map_err(|error| error.to_string())?;
        }
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        let pieces: Vec<_> = line
            .trim_end_matches(['\n', '\r'])
            .split_whitespace()
            .collect();
        if pieces.len() < numeric.len() {
            return Err("input did not provide a value for every destination".to_owned());
        }
        pieces
            .iter()
            .take(numeric.len())
            .zip(numeric)
            .map(|(piece, wants_number)| {
                if *wants_number {
                    piece
                        .parse::<f64>()
                        .map(Value::Number)
                        .map_err(|error| error.to_string())
                } else {
                    Ok(Value::String((*piece).to_owned()))
                }
            })
            .collect()
    }
    fn host_call(
        &mut self,
        receiver: Option<Value>,
        name: &str,
        arguments: Vec<Value>,
    ) -> Result<Value, String> {
        let _ = (receiver, arguments);
        Err(format!("host operation `{name}` is not supported"))
    }
    fn load_unit(&mut self, file_name: &str) -> Result<Option<String>, String> {
        match std::fs::read_to_string(file_name) {
            Ok(source) => Ok(Some(source)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
    fn launch(&mut self, procedure: &str, arguments: Vec<Value>) -> Result<(), String> {
        let _ = arguments;
        Err(format!(
            "asynchronous procedure launch `{procedure}` is not supported by the host"
        ))
    }
    fn register_event(&mut self, event: &str, procedure: &str) -> Result<(), String> {
        Err(format!(
            "event registration `{event}` -> `{procedure}` is not supported by the host"
        ))
    }
    fn create_channel(&mut self) -> Result<u64, String> {
        Err("channels are not supported by the host".to_owned())
    }
    fn send(&mut self, channel: u64, value: Option<Value>) -> Result<(), String> {
        let _ = value;
        Err(format!(
            "send on channel {channel} is not supported by the host"
        ))
    }
    fn receive(&mut self, channel: u64) -> Result<Option<Value>, String> {
        Err(format!(
            "receive on channel {channel} is not supported by the host"
        ))
    }
    fn select(&mut self, operations: &[ChannelOperation]) -> Result<SelectedOperation, String> {
        let _ = operations;
        Err("channel selection is not supported by the host".to_owned())
    }
    fn write_file(&mut self, file: Value, values: &[Value], newline: bool) -> Result<(), String> {
        self.host_call(
            Some(file),
            if newline { "write_line" } else { "write" },
            values.to_vec(),
        )
        .map(|_| ())
    }
    fn read_file(&mut self, file: Value, count: usize) -> Result<Vec<Value>, String> {
        self.host_call(Some(file), "read_values", vec![Value::Number(count as f64)])
            .and_then(|value| match value {
                Value::Array(values) => Ok(values.borrow().clone()),
                _ => Err("host file read must return an array".to_owned()),
            })
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    parsed: syntax::ParsedProgram,
}

pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    syntax::parse(source).map(|parsed| Program { parsed })
}

#[derive(Default)]
struct Frame {
    locals: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
enum Flow {
    Continue,
    Return(Option<Value>, Span),
    TailCall(String, Vec<Value>, Span),
    Goto(String, Span),
}

pub struct Interpreter<H: Host> {
    host: H,
    steps: u64,
    depth: usize,
    globals: HashMap<String, Value>,
    shared_names: HashSet<String>,
    procedures: HashMap<String, syntax::Procedure>,
    functions: HashMap<String, syntax::Procedure>,
    classes: HashMap<String, syntax::ClassDef>,
    methods: Vec<syntax::MethodDef>,
    event_handlers: HashMap<String, String>,
}

impl<H: Host> Interpreter<H> {
    pub fn new(host: H) -> Self {
        Self {
            host,
            steps: 0,
            depth: 0,
            globals: HashMap::new(),
            shared_names: HashSet::new(),
            procedures: HashMap::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            methods: Vec::new(),
            event_handlers: HashMap::new(),
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<Execution, RuntimeError> {
        self.steps = 0;
        self.depth = 0;
        self.globals.clear();
        self.shared_names.clear();
        self.event_handlers.clear();
        let mut parsed = program.parsed.clone();
        let mut loaded = HashSet::new();
        self.load_units(&mut parsed, &mut loaded)?;
        self.procedures = parsed.procedures;
        self.functions = parsed.functions;
        self.classes = parsed.classes;
        self.methods = parsed.methods;
        let mut frame = Frame::default();
        for statement in &parsed.unit_globals {
            match &statement.kind {
                syntax::StmtKind::Assign {
                    target:
                        syntax::Target {
                            kind: syntax::TargetKind::Variable(name),
                            ..
                        },
                    value,
                } => {
                    let value = self.evaluate(value, &mut frame)?;
                    if self.globals.contains_key(name) {
                        return Err(runtime_error_at(
                            &statement.span,
                            format!("duplicate unit global `{name}`"),
                        ));
                    }
                    self.globals.insert(name.clone(), value);
                    self.shared_names.insert(name.clone());
                }
                _ => {
                    return Err(runtime_error_at(
                        &statement.span,
                        "units may only initialize global variables",
                    ));
                }
            }
        }
        match self.execute_block(&parsed.main, &mut frame, true)? {
            Flow::Continue => Ok(Execution { steps: self.steps }),
            Flow::Return(_, span) => Err(runtime_error_at(
                &span,
                "return used outside a procedure or function",
            )),
            Flow::TailCall(_, _, span) => Err(runtime_error_at(
                &span,
                "tail call used outside a procedure or function",
            )),
            Flow::Goto(name, span) => {
                Err(runtime_error_at(&span, format!("undefined label `{name}`")))
            }
        }
    }

    pub fn dispatch_event(
        &mut self,
        event: &str,
        arguments: Vec<Value>,
    ) -> Result<Option<Value>, RuntimeError> {
        self.steps = 0;
        self.depth = 0;
        let procedure = self.event_handlers.get(event).cloned().ok_or_else(|| {
            runtime_error(format!("no handler is registered for event `{event}`"))
        })?;
        self.invoke(&procedure, arguments, Span::default(), false)
    }

    pub fn host(&self) -> &H {
        &self.host
    }
    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    fn load_units(
        &mut self,
        parsed: &mut syntax::ParsedProgram,
        loaded: &mut HashSet<String>,
    ) -> Result<(), RuntimeError> {
        let names = std::mem::take(&mut parsed.units);
        for file_name in names {
            if !loaded.insert(file_name.clone()) {
                continue;
            }
            let source = self
                .host
                .load_unit(&file_name)
                .map_err(|error| runtime_error(format!("loading unit `{file_name}`: {error}")))?
                .ok_or_else(|| runtime_error(format!("unit `{file_name}` was not found")))?;
            let mut unit = syntax::parse(&source).map_err(|errors| {
                runtime_error(format!(
                    "unit `{file_name}`: {}",
                    errors
                        .into_iter()
                        .map(|error| error.to_string())
                        .collect::<Vec<_>>()
                        .join("; ")
                ))
            })?;
            if !unit.main.is_empty() {
                return Err(runtime_error(format!(
                    "unit `{file_name}` contains executable statements"
                )));
            }
            self.load_units(&mut unit, loaded)?;
            for (name, procedure) in unit.procedures {
                if parsed.functions.contains_key(&name) {
                    return Err(runtime_error(format!(
                        "duplicate procedure or function `{name}` while loading unit `{file_name}`"
                    )));
                }
                if parsed.procedures.insert(name.clone(), procedure).is_some() {
                    return Err(runtime_error(format!(
                        "duplicate procedure `{name}` while loading unit `{file_name}`"
                    )));
                }
            }
            for (name, function) in unit.functions {
                if parsed.procedures.contains_key(&name) {
                    return Err(runtime_error(format!(
                        "duplicate procedure or function `{name}` while loading unit `{file_name}`"
                    )));
                }
                if parsed.functions.insert(name.clone(), function).is_some() {
                    return Err(runtime_error(format!(
                        "duplicate function `{name}` while loading unit `{file_name}`"
                    )));
                }
            }
            for (name, class) in unit.classes {
                if parsed.classes.insert(name.clone(), class).is_some() {
                    return Err(runtime_error(format!(
                        "duplicate class `{name}` while loading unit `{file_name}`"
                    )));
                }
            }
            for method in unit.methods {
                if parsed.methods.iter().any(|existing| {
                    existing.class_name == method.class_name && existing.name == method.name
                }) {
                    return Err(runtime_error(format!(
                        "duplicate method `{}:{}` while loading unit `{file_name}`",
                        method.class_name, method.name
                    )));
                }
                parsed.methods.push(method);
            }
            parsed.unit_globals.extend(unit.unit_globals);
        }
        Ok(())
    }

    fn tick(&mut self, span: Span) -> Result<(), RuntimeError> {
        self.steps = self
            .steps
            .checked_add(1)
            .filter(|steps| *steps <= MAX_STEPS)
            .ok_or_else(|| runtime_error_at(&span, "execution step limit exceeded"))?;
        Ok(())
    }

    fn execute_block(
        &mut self,
        statements: &[syntax::Stmt],
        frame: &mut Frame,
        main: bool,
    ) -> Result<Flow, RuntimeError> {
        let mut labels = HashMap::new();
        for (index, statement) in statements.iter().enumerate() {
            if let syntax::StmtKind::Label(name) = &statement.kind {
                labels.insert(name.clone(), index);
            }
        }
        let mut at = 0usize;
        while at < statements.len() {
            let statement = &statements[at];
            self.tick(statement.span)?;
            let flow = self.execute_statement(statement, frame, main)?;
            match flow {
                Flow::Continue => at += 1,
                Flow::Goto(name, span) => {
                    if let Some(target) = labels.get(&name) {
                        at = *target;
                    } else {
                        return Ok(Flow::Goto(name, span));
                    }
                }
                other => return Ok(other),
            }
        }
        Ok(Flow::Continue)
    }

    fn execute_statement(
        &mut self,
        statement: &syntax::Stmt,
        frame: &mut Frame,
        main: bool,
    ) -> Result<Flow, RuntimeError> {
        use syntax::StmtKind as S;
        let span = statement.span;
        match &statement.kind {
            S::Assign { target, value } => {
                let value = self.evaluate(value, frame)?;
                self.assign_target(target, value, frame)?;
            }
            S::Read { prompt, targets } => {
                let prompt = prompt
                    .as_ref()
                    .map(|expr| self.evaluate(expr, frame).map(|value| value.display_text()))
                    .transpose()?;
                let flags: Vec<_> = targets.iter().map(|(_, numeric)| *numeric).collect();
                let values = self
                    .host
                    .read_input(prompt.as_deref(), &flags)
                    .map_err(|error| runtime_error_at(&span, error))?;
                if values.len() != targets.len() {
                    return Err(runtime_error_at(
                        &span,
                        "host returned the wrong number of input values",
                    ));
                }
                for ((target, numeric), value) in targets.iter().zip(values) {
                    let value = if *numeric {
                        coerce_number(value).map_err(|error| runtime_error_at(&span, error))?
                    } else {
                        value
                    };
                    self.assign_target(target, value, frame)?;
                }
            }
            S::ReadFile { file, targets } => {
                let file = self.evaluate(file, frame)?;
                let values = self
                    .host
                    .read_file(file, targets.len())
                    .map_err(|error| runtime_error_at(&span, error))?;
                if values.len() != targets.len() {
                    return Err(runtime_error_at(
                        &span,
                        "host returned the wrong number of file values",
                    ));
                }
                for (target, value) in targets.iter().zip(values) {
                    self.assign_target(target, value, frame)?;
                }
            }
            S::Print { values, newline } => {
                let values = self.evaluate_all(values, frame)?;
                self.host
                    .write_output(&values, *newline)
                    .map_err(|error| runtime_error_at(&span, error))?;
            }
            S::PrintFile {
                file,
                values,
                newline,
            } => {
                let file = self.evaluate(file, frame)?;
                let values = self.evaluate_all(values, frame)?;
                self.host
                    .write_file(file, &values, *newline)
                    .map_err(|error| runtime_error_at(&span, error))?;
            }
            S::Graphics { command, arguments } => {
                let values = arguments
                    .iter()
                    .map(|argument| {
                        argument
                            .as_ref()
                            .map(|expression| self.evaluate(expression, frame))
                            .transpose()
                            .map(|value| value.unwrap_or(Value::Uninitialized))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let canonical = canonical_builtin(command);
                validate_graphics_arguments(&canonical, &values, span)?;
                if self.call_builtin(command, &values, frame, span)?.is_none() {
                    self.host
                        .host_call(None, &canonical, values)
                        .map_err(|error| runtime_error_at(&span, error))?;
                }
            }
            S::If {
                branches,
                otherwise,
            } => {
                for (condition, body) in branches {
                    if Self::truthy(self.evaluate(condition, frame)?, condition.span)? {
                        return self.execute_block(body, frame, main);
                    }
                }
                return self.execute_block(otherwise, frame, main);
            }
            S::While { condition, body } => {
                while Self::truthy(self.evaluate(condition, frame)?, condition.span)? {
                    self.tick(span)?;
                    match self.execute_block(body, frame, main)? {
                        Flow::Continue => {}
                        flow => return Ok(flow),
                    }
                }
            }
            S::Loop { body } => loop {
                self.tick(span)?;
                match self.execute_block(body, frame, main)? {
                    Flow::Continue => {}
                    flow => return Ok(flow),
                }
            },
            S::For {
                variable,
                start,
                end,
                step,
                descending,
                body,
            } => {
                let first = require_integer(self.evaluate(start, frame)?, start.span)?;
                let last = require_integer(self.evaluate(end, frame)?, end.span)?;
                let stride = if let Some(step) = step {
                    require_integer(self.evaluate(step, frame)?, step.span)?
                } else if *descending {
                    -1
                } else {
                    1
                };
                if stride == 0 || (*descending && stride > 0) || (!*descending && stride < 0) {
                    return Err(runtime_error_at(
                        &span,
                        "counted-loop step must be nonzero and match its direction",
                    ));
                }
                let mut value = first;
                while if *descending {
                    value >= last
                } else {
                    value <= last
                } {
                    frame
                        .locals
                        .insert(variable.clone(), Value::Number(value as f64));
                    self.tick(span)?;
                    match self.execute_block(body, frame, main)? {
                        Flow::Continue => {}
                        flow => return Ok(flow),
                    }
                    if value == last {
                        break;
                    }
                    let Some(next) = value.checked_add(stride) else {
                        break;
                    };
                    value = next;
                }
            }
            S::Label(_) => {}
            S::Goto(name) => return Ok(Flow::Goto(name.clone(), span)),
            S::Expression(expression) => {
                let _ = self.evaluate(expression, frame)?;
            }
            S::Return(value) => {
                return Ok(Flow::Return(
                    value
                        .as_ref()
                        .map(|expr| self.evaluate(expr, frame))
                        .transpose()?,
                    span,
                ));
            }
            S::TailCall { name, arguments } => {
                return Ok(Flow::TailCall(
                    name.clone(),
                    self.evaluate_all(arguments, frame)?,
                    span,
                ));
            }
            S::Launch { name, arguments } => {
                let values = self.evaluate_all(arguments, frame)?;
                let procedure = self.procedures.get(name).ok_or_else(|| {
                    runtime_error_at(&span, format!("undefined launch procedure `{name}`"))
                })?;
                if procedure.parameters.len() != values.len() {
                    return Err(runtime_error_at(
                        &span,
                        format!(
                            "launch of `{name}` expects {} argument(s), got {}",
                            procedure.parameters.len(),
                            values.len()
                        ),
                    ));
                }
                self.host
                    .launch(name, values)
                    .map_err(|error| runtime_error_at(&span, error))?;
            }
            S::Send { channel, value } => {
                let channel = Self::channel_id(self.evaluate(channel, frame)?, span)?;
                let value = value
                    .as_ref()
                    .map(|expr| self.evaluate(expr, frame))
                    .transpose()?;
                self.host
                    .send(channel, value)
                    .map_err(|error| runtime_error_at(&span, error))?;
            }
            S::Receive { channel, target } => {
                let channel = Self::channel_id(self.evaluate(channel, frame)?, span)?;
                let value = self
                    .host
                    .receive(channel)
                    .map_err(|error| runtime_error_at(&span, error))?;
                if let Some(target) = target {
                    self.assign_target(target, value.unwrap_or(Value::Uninitialized), frame)?;
                }
            }
            S::Select(branches) => {
                let mut operations = Vec::new();
                for branch in branches {
                    operations.push(match &branch.operation {
                        syntax::SelectOperation::Send { channel, value } => {
                            ChannelOperation::Send {
                                channel: self.evaluate(channel, frame)?,
                                value: value
                                    .as_ref()
                                    .map(|expr| self.evaluate(expr, frame))
                                    .transpose()?,
                            }
                        }
                        syntax::SelectOperation::Receive { channel, .. } => {
                            ChannelOperation::Receive {
                                channel: self.evaluate(channel, frame)?,
                            }
                        }
                    });
                }
                let selected = self
                    .host
                    .select(&operations)
                    .map_err(|error| runtime_error_at(&span, error))?;
                let branch = branches.get(selected.index).ok_or_else(|| {
                    runtime_error_at(&span, "host selected an invalid channel operation")
                })?;
                self.tick(branch.span)?;
                if let syntax::SelectOperation::Receive {
                    target: Some(target),
                    ..
                } = &branch.operation
                {
                    self.assign_target(
                        target,
                        selected.received.unwrap_or(Value::Uninitialized),
                        frame,
                    )?;
                }
                return self.execute_block(&branch.body, frame, main);
            }
            S::RegisterEvent { event, procedure } => {
                if !self.procedures.contains_key(procedure) {
                    return Err(runtime_error_at(
                        &span,
                        format!("undefined event procedure `{procedure}`"),
                    ));
                }
                self.host
                    .register_event(event, procedure)
                    .map_err(|error| runtime_error_at(&span, error))?;
                self.event_handlers.insert(event.clone(), procedure.clone());
            }
            S::Shared(name) => {
                if !main {
                    return Err(runtime_error_at(
                        &span,
                        "shared declarations are only supported in the main program",
                    ));
                }
                if let Some(value) = frame.locals.remove(name) {
                    self.globals.insert(name.clone(), value);
                } else {
                    self.globals
                        .entry(name.clone())
                        .or_insert(Value::Uninitialized);
                }
                self.shared_names.insert(name.clone());
            }
            S::UseUnit(_) => {}
        }
        Ok(Flow::Continue)
    }

    fn evaluate_all(
        &mut self,
        expressions: &[syntax::Expr],
        frame: &mut Frame,
    ) -> Result<Vec<Value>, RuntimeError> {
        expressions
            .iter()
            .map(|expression| self.evaluate(expression, frame))
            .collect()
    }

    fn channel_id(value: Value, span: Span) -> Result<u64, RuntimeError> {
        match value {
            Value::Channel(id) => Ok(id),
            _ => Err(runtime_error_at(&span, "expected a channel value")),
        }
    }

    fn assign_target(
        &mut self,
        target: &syntax::Target,
        value: Value,
        frame: &mut Frame,
    ) -> Result<(), RuntimeError> {
        use syntax::TargetKind as T;
        match &target.kind {
            T::Variable(name) => self.assign_variable(name, value, frame),
            T::Index { object, indices } => {
                let object = self.evaluate(object, frame)?;
                let index_values = self.evaluate_all(indices, frame)?;
                self.set_index(object, &index_values, value, target.span)
            }
            T::Field { object, name } => {
                let object = self.evaluate(object, frame)?;
                match object {
                    Value::Object(object) => {
                        object.borrow_mut().fields.insert(name.clone(), value);
                        Ok(())
                    }
                    other => Err(runtime_error_at(
                        &target.span,
                        format!("cannot assign field `{name}` on {}", other.kind_name()),
                    )),
                }
            }
        }
    }

    fn assign_variable(
        &mut self,
        name: &str,
        value: Value,
        frame: &mut Frame,
    ) -> Result<(), RuntimeError> {
        if frame.locals.contains_key(name) || !self.shared_names.contains(name) {
            frame.locals.insert(name.to_owned(), value);
        } else {
            self.globals.insert(name.to_owned(), value);
        }
        Ok(())
    }

    fn evaluate(
        &mut self,
        expression: &syntax::Expr,
        frame: &mut Frame,
    ) -> Result<Value, RuntimeError> {
        use syntax::ExprKind as E;
        let span = expression.span;
        match &expression.kind {
            E::Number(number) => Ok(Value::Number(*number)),
            E::String(value) => Ok(Value::String(value.clone())),
            E::Boolean(value) => Ok(Value::Boolean(*value)),
            E::Variable(name) => self.lookup_variable(name, frame, span),
            E::Array(items) => Ok(Value::Array(Rc::new(RefCell::new(
                self.evaluate_all(items, frame)?,
            )))),
            E::Dictionary(entries) => {
                let mut values = Vec::with_capacity(entries.len());
                for (key_expression, value_expression) in entries {
                    let key = self.evaluate(key_expression, frame)?;
                    validate_dictionary_key(&key)
                        .map_err(|error| runtime_error_at(&key_expression.span, error))?;
                    let value = self.evaluate(value_expression, frame)?;
                    values.push((key, value));
                }
                Ok(Value::Dictionary(Rc::new(RefCell::new(values))))
            }
            E::Unary { operator, value } => {
                let value = self.evaluate(value, frame)?;
                match operator {
                    syntax::UnaryOp::Negate => value
                        .number()
                        .map(|number| Value::Number(-number))
                        .ok_or_else(|| runtime_error_at(&span, "unary minus expects a number")),
                    syntax::UnaryOp::Not => {
                        Self::truthy(value, span).map(|truth| Value::Boolean(!truth))
                    }
                }
            }
            E::Binary {
                left,
                operator,
                right,
            } => {
                let left_value = self.evaluate(left, frame)?;
                match operator {
                    syntax::BinaryOp::And => {
                        if !Self::truthy(left_value, left.span)? {
                            return Ok(Value::Boolean(false));
                        }
                        return Self::truthy(self.evaluate(right, frame)?, right.span)
                            .map(Value::Boolean);
                    }
                    syntax::BinaryOp::Or => {
                        if Self::truthy(left_value, left.span)? {
                            return Ok(Value::Boolean(true));
                        }
                        return Self::truthy(self.evaluate(right, frame)?, right.span)
                            .map(Value::Boolean);
                    }
                    syntax::BinaryOp::Nor => {
                        if Self::truthy(left_value, left.span)? {
                            return Ok(Value::Boolean(false));
                        }
                        return Self::truthy(self.evaluate(right, frame)?, right.span)
                            .map(|right| Value::Boolean(!right));
                    }
                    _ => {}
                }
                let right_value = self.evaluate(right, frame)?;
                apply_binary(left_value, *operator, right_value, span, &self.classes)
            }
            E::Call { name, arguments } => {
                let values = self.evaluate_all(arguments, frame)?;
                self.call_expression(name, values, frame, span)
            }
            E::Index { object, indices } => {
                let object = self.evaluate(object, frame)?;
                let indices = self.evaluate_all(indices, frame)?;
                self.get_index(object, &indices, span)
            }
            E::Field { object, name } => {
                let object = self.evaluate(object, frame)?;
                match object {
                    Value::Object(object) => object
                        .borrow()
                        .fields
                        .get(name)
                        .cloned()
                        .filter(|value| !matches!(value, Value::Uninitialized))
                        .ok_or_else(|| {
                            runtime_error_at(&span, format!("undefined field `{name}`"))
                        }),
                    other => Err(runtime_error_at(
                        &span,
                        format!("cannot read field `{name}` from {}", other.kind_name()),
                    )),
                }
            }
            E::MethodCall {
                object,
                name,
                arguments,
            } => {
                let receiver = self.evaluate(object, frame)?;
                let arguments = self.evaluate_all(arguments, frame)?;
                self.call_method(receiver, name, arguments, frame, span)
            }
            E::NewObject(name) => self.new_object(name, span),
            E::NewWidget(name) => self
                .host
                .host_call(None, "new_widget", vec![Value::String(name.clone())])
                .map_err(|error| runtime_error_at(&span, error)),
        }
    }

    fn lookup_variable(
        &self,
        name: &str,
        frame: &Frame,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Some(value) = frame.locals.get(name) {
            return if matches!(value, Value::Uninitialized) {
                Err(runtime_error_at(
                    &span,
                    format!("variable `{name}` has no value"),
                ))
            } else {
                Ok(value.clone())
            };
        }
        if let Some(value) = self.globals.get(name) {
            return if matches!(value, Value::Uninitialized) {
                Err(runtime_error_at(
                    &span,
                    format!("shared variable `{name}` has no value"),
                ))
            } else {
                Ok(value.clone())
            };
        }
        if self.classes.contains_key(name) {
            return Ok(Value::Class(name.to_owned()));
        }
        if is_type_name(name) {
            return Ok(Value::TypeName(name.to_owned()));
        }
        Err(runtime_error_at(
            &span,
            format!("undefined variable `{name}`"),
        ))
    }

    fn truthy(value: Value, span: Span) -> Result<bool, RuntimeError> {
        match value {
            Value::Boolean(value) => Ok(value),
            other => Err(runtime_error_at(
                &span,
                format!("condition must be boolean, got {}", other.kind_name()),
            )),
        }
    }

    fn call_expression(
        &mut self,
        name: &str,
        arguments: Vec<Value>,
        frame: &mut Frame,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if self.functions.contains_key(name) {
            return self
                .invoke(name, arguments, span, true)
                .map(|value| value.unwrap_or(Value::Uninitialized));
        }
        if self.procedures.contains_key(name) {
            return self
                .invoke(name, arguments, span, false)
                .map(|value| value.unwrap_or(Value::Uninitialized));
        }
        if let Some(value) = self.call_builtin(name, &arguments, frame, span)? {
            return Ok(value);
        }
        self.host
            .host_call(None, name, arguments)
            .map_err(|error| runtime_error_at(&span, error))
    }

    fn call_method(
        &mut self,
        receiver: Value,
        name: &str,
        arguments: Vec<Value>,
        frame: &mut Frame,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Value::Object(object) = &receiver {
            let class_name = object.borrow().class_name.clone();
            if let Some(method) = self.find_method(&class_name, name).cloned() {
                if method.parameters.len() != arguments.len() {
                    return Err(runtime_error_at(
                        &span,
                        format!(
                            "method `{name}` expects {} argument(s), got {}",
                            method.parameters.len(),
                            arguments.len()
                        ),
                    ));
                }
                self.depth += 1;
                if self.depth > MAX_CALL_DEPTH {
                    self.depth -= 1;
                    return Err(runtime_error_at(&span, "maximum call depth exceeded"));
                }
                let result = (|| {
                    let mut method_frame = Frame::default();
                    method_frame
                        .locals
                        .insert(method.receiver.clone(), receiver.clone());
                    for (parameter, value) in method.parameters.iter().zip(arguments) {
                        method_frame.locals.insert(parameter.clone(), value);
                    }
                    match self.execute_block(&method.body, &mut method_frame, false)? {
                        Flow::Return(value, return_span) => {
                            if method.returns_value {
                                value.ok_or_else(|| {
                                    runtime_error_at(&return_span, "method returned no value")
                                })
                            } else {
                                Ok(Value::Uninitialized)
                            }
                        }
                        Flow::Continue if method.returns_value => Err(runtime_error_at(
                            &method.span,
                            format!("method `{name}` did not return a value"),
                        )),
                        Flow::Continue => Ok(Value::Uninitialized),
                        Flow::TailCall(tail, values, tail_span) => self
                            .invoke(&tail, values, tail_span, method.returns_value)
                            .map(|value| value.unwrap_or(Value::Uninitialized)),
                        Flow::Goto(label, goto_span) => Err(runtime_error_at(
                            &goto_span,
                            format!("undefined label `{label}`"),
                        )),
                    }
                })();
                self.depth = self.depth.saturating_sub(1);
                return result;
            }
            if self.classes.contains_key(&class_name) {
                if let Some(signature) = self.find_method_signature(&class_name, name) {
                    if signature.parameters.len() != arguments.len() {
                        return Err(runtime_error_at(
                            &self.classes[&class_name].span,
                            format!(
                                "method `{name}` expects {} argument(s), got {}",
                                signature.parameters.len(),
                                arguments.len()
                            ),
                        ));
                    }
                    let _returns_value = signature.returns_value;
                    return Err(runtime_error_at(
                        &self.classes[&class_name].span,
                        format!("declared method `{}` has no implementation", signature.name),
                    ));
                }
                return Err(runtime_error_at(
                    &span,
                    format!("class `{class_name}` has no method `{name}`"),
                ));
            }
        }
        if let Some(value) = self.call_builtin(name, &arguments, frame, span)? {
            return Ok(value);
        }
        self.host
            .host_call(Some(receiver), name, arguments)
            .map_err(|error| runtime_error_at(&span, error))
    }

    fn invoke(
        &mut self,
        initial_name: &str,
        initial_arguments: Vec<Value>,
        span: Span,
        must_return: bool,
    ) -> Result<Option<Value>, RuntimeError> {
        let mut name = initial_name.to_owned();
        let mut arguments = initial_arguments;
        let mut call_span = span;
        loop {
            let (procedure, returns_value) = if let Some(function) = self.functions.get(&name) {
                (function.clone(), true)
            } else if let Some(procedure) = self.procedures.get(&name) {
                (procedure.clone(), false)
            } else {
                return Err(runtime_error_at(
                    &call_span,
                    format!("undefined procedure or function `{name}`"),
                ));
            };
            if procedure.parameters.len() != arguments.len() {
                return Err(runtime_error_at(
                    &call_span,
                    format!(
                        "`{name}` expects {} argument(s), got {}",
                        procedure.parameters.len(),
                        arguments.len()
                    ),
                ));
            }
            self.depth += 1;
            if self.depth > MAX_CALL_DEPTH {
                self.depth -= 1;
                return Err(runtime_error_at(&call_span, "maximum call depth exceeded"));
            }
            let mut frame = Frame::default();
            for (parameter, value) in procedure.parameters.iter().zip(arguments.drain(..)) {
                frame.locals.insert(parameter.clone(), value);
            }
            let flow = self.execute_block(&procedure.body, &mut frame, false);
            self.depth = self.depth.saturating_sub(1);
            match flow? {
                Flow::Continue if returns_value || must_return => {
                    return Err(runtime_error_at(
                        &procedure.span,
                        format!("function `{name}` did not return a value"),
                    ));
                }
                Flow::Continue => return Ok(None),
                Flow::Return(value, return_span) => {
                    if returns_value || must_return {
                        return value.map(Some).ok_or_else(|| {
                            runtime_error_at(
                                &return_span,
                                format!("function `{name}` returned no value"),
                            )
                        });
                    }
                    return Ok(None);
                }
                Flow::TailCall(next, values, tail_span) => {
                    name = next;
                    arguments = values;
                    call_span = tail_span;
                }
                Flow::Goto(label, goto_span) => {
                    return Err(runtime_error_at(
                        &goto_span,
                        format!("undefined label `{label}`"),
                    ));
                }
            }
        }
    }

    fn find_method(&self, class_name: &str, name: &str) -> Option<&syntax::MethodDef> {
        self.find_method_inner(class_name, name, &mut HashSet::new())
    }

    fn find_method_inner(
        &self,
        class_name: &str,
        name: &str,
        seen: &mut HashSet<String>,
    ) -> Option<&syntax::MethodDef> {
        if !seen.insert(class_name.to_owned()) {
            return None;
        }
        self.methods
            .iter()
            .find(|method| method.class_name == class_name && method.name == name)
            .or_else(|| {
                self.classes.get(class_name).and_then(|class| {
                    class
                        .bases
                        .iter()
                        .find_map(|base| self.find_method_inner(base, name, seen))
                })
            })
    }

    fn find_method_signature(
        &self,
        class_name: &str,
        name: &str,
    ) -> Option<&syntax::MethodSignature> {
        self.find_method_signature_inner(class_name, name, &mut HashSet::new())
    }

    fn find_method_signature_inner(
        &self,
        class_name: &str,
        name: &str,
        seen: &mut HashSet<String>,
    ) -> Option<&syntax::MethodSignature> {
        if !seen.insert(class_name.to_owned()) {
            return None;
        }
        self.classes.get(class_name).and_then(|class| {
            class
                .declared_methods
                .iter()
                .find(|method| method.name == name)
                .or_else(|| {
                    class
                        .bases
                        .iter()
                        .find_map(|base| self.find_method_signature_inner(base, name, seen))
                })
        })
    }

    fn new_object(&self, name: &str, span: Span) -> Result<Value, RuntimeError> {
        if !self.classes.contains_key(name) {
            return Err(runtime_error_at(&span, format!("undefined class `{name}`")));
        }
        self.validate_bases(name, &mut HashSet::new())?;
        let mut fields = HashMap::new();
        self.collect_fields(name, &mut fields, &mut HashSet::new());
        Ok(Value::Object(Rc::new(RefCell::new(ObjectValue {
            class_name: name.to_owned(),
            fields,
        }))))
    }

    fn validate_bases(&self, name: &str, seen: &mut HashSet<String>) -> Result<(), RuntimeError> {
        if !seen.insert(name.to_owned()) {
            return Err(runtime_error_at(
                &self.classes[name].span,
                format!("inheritance cycle includes `{name}`"),
            ));
        }
        let class = &self.classes[name];
        for base in &class.bases {
            if !self.classes.contains_key(base) {
                return Err(runtime_error_at(
                    &class.span,
                    format!("undefined base class `{base}`"),
                ));
            }
            self.validate_bases(base, seen)?;
        }
        seen.remove(name);
        Ok(())
    }

    fn collect_fields(
        &self,
        name: &str,
        fields: &mut HashMap<String, Value>,
        seen: &mut HashSet<String>,
    ) {
        if !seen.insert(name.to_owned()) {
            return;
        }
        if let Some(class) = self.classes.get(name) {
            for base in &class.bases {
                self.collect_fields(base, fields, seen);
            }
            for field in &class.fields {
                fields.entry(field.clone()).or_insert(Value::Uninitialized);
            }
        }
    }

    fn get_index(
        &self,
        object: Value,
        indices: &[Value],
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if indices.is_empty() {
            return Err(runtime_error_at(&span, "index expects at least one value"));
        }
        let mut current = object;
        for index in indices {
            current = match current {
                Value::Array(values) => {
                    let index = require_integer(index.clone(), span)?;
                    if index < 1 {
                        return Err(runtime_error_at(&span, "array indices start at one"));
                    }
                    values
                        .borrow()
                        .get((index - 1) as usize)
                        .cloned()
                        .ok_or_else(|| runtime_error_at(&span, "array index is out of range"))?
                }
                Value::Dictionary(values) => {
                    let value = values
                        .borrow()
                        .iter()
                        .find(|(key, _)| dictionary_key_equal(key, index))
                        .map(|(_, value)| value.clone());
                    value.ok_or_else(|| runtime_error_at(&span, "dictionary key does not exist"))?
                }
                other => {
                    return Err(runtime_error_at(
                        &span,
                        format!("cannot index {}", other.kind_name()),
                    ));
                }
            };
        }
        if matches!(current, Value::Uninitialized) {
            return Err(runtime_error_at(&span, "array element has no value"));
        }
        Ok(current)
    }

    fn set_index(
        &mut self,
        object: Value,
        indices: &[Value],
        value: Value,
        span: Span,
    ) -> Result<(), RuntimeError> {
        if indices.is_empty() {
            return Err(runtime_error_at(&span, "index expects at least one value"));
        }
        let mut current = object;
        for index in &indices[..indices.len() - 1] {
            current = match current {
                Value::Array(values) => {
                    let index = require_integer(index.clone(), span)?;
                    if index < 1 {
                        return Err(runtime_error_at(&span, "array indices start at one"));
                    }
                    values
                        .borrow()
                        .get((index - 1) as usize)
                        .cloned()
                        .ok_or_else(|| runtime_error_at(&span, "array index is out of range"))?
                }
                Value::Dictionary(values) => values
                    .borrow()
                    .iter()
                    .find(|(key, _)| dictionary_key_equal(key, index))
                    .map(|(_, value)| value.clone())
                    .ok_or_else(|| runtime_error_at(&span, "dictionary key does not exist"))?,
                other => {
                    return Err(runtime_error_at(
                        &span,
                        format!("cannot index {}", other.kind_name()),
                    ));
                }
            };
        }
        let last = indices.last().expect("nonempty indices");
        match current {
            Value::Array(values) => {
                let index = require_integer(last.clone(), span)?;
                if index < 1 {
                    return Err(runtime_error_at(&span, "array indices start at one"));
                }
                let mut values = values.borrow_mut();
                let slot = values
                    .get_mut((index - 1) as usize)
                    .ok_or_else(|| runtime_error_at(&span, "array index is out of range"))?;
                *slot = value;
                Ok(())
            }
            Value::Dictionary(values) => {
                validate_dictionary_key(last).map_err(|error| runtime_error_at(&span, error))?;
                let mut values = values.borrow_mut();
                if let Some((_, stored)) = values
                    .iter_mut()
                    .find(|(key, _)| dictionary_key_equal(key, last))
                {
                    *stored = value;
                } else {
                    values.push((last.clone(), value));
                }
                Ok(())
            }
            other => Err(runtime_error_at(
                &span,
                format!("cannot assign index on {}", other.kind_name()),
            )),
        }
    }

    fn call_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        _frame: &mut Frame,
        span: Span,
    ) -> Result<Option<Value>, RuntimeError> {
        let canonical = canonical_builtin(name);
        let argument = |index: usize| {
            arguments.get(index).ok_or_else(|| {
                runtime_error_at(&span, format!("`{name}` is missing argument {}", index + 1))
            })
        };
        let arity = |minimum: usize, maximum: usize| -> Result<(), RuntimeError> {
            if arguments.len() < minimum || arguments.len() > maximum {
                let expected = if minimum == maximum {
                    minimum.to_string()
                } else {
                    format!("{minimum}..={maximum}")
                };
                Err(runtime_error_at(
                    &span,
                    format!(
                        "`{name}` expects {expected} argument(s), got {}",
                        arguments.len()
                    ),
                ))
            } else {
                Ok(())
            }
        };
        let number = |index: usize| -> Result<f64, RuntimeError> {
            argument(index)?.number().ok_or_else(|| {
                runtime_error_at(
                    &span,
                    format!("argument {} to `{name}` must be a number", index + 1),
                )
            })
        };
        let text = |index: usize| -> Result<&str, RuntimeError> {
            match argument(index)? {
                Value::String(value) => Ok(value),
                other => Err(runtime_error_at(
                    &span,
                    format!(
                        "argument {} to `{name}` must be a string, got {}",
                        index + 1,
                        other.kind_name()
                    ),
                )),
            }
        };
        let result = match canonical.as_str() {
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sqrt" | "log10" | "ln"
            | "round" => {
                arity(1, 1)?;
                let input = number(0)?;
                let output = match canonical.as_str() {
                    "sin" => input.sin(),
                    "cos" => input.cos(),
                    "tan" => input.tan(),
                    "asin" => input.asin(),
                    "acos" => input.acos(),
                    "atan" => input.atan(),
                    "sqrt" => input.sqrt(),
                    "log10" => input.log10(),
                    "ln" => input.ln(),
                    _ => input.round(),
                };
                Some(Value::Number(output))
            }
            "remainder" => {
                arity(2, 2)?;
                Some(Value::Number(number(0)? % number(1)?))
            }
            "random" => {
                arity(1, 1)?;
                let bound = require_integer(argument(0)?.clone(), span)?;
                if bound <= 0 {
                    return Err(runtime_error_at(&span, "random bound must be positive"));
                }
                let value = self
                    .host
                    .random(bound)
                    .map_err(|error| runtime_error_at(&span, error))?;
                if value < 0 || value >= bound {
                    return Err(runtime_error_at(
                        &span,
                        "host random value is outside the documented range [0, bound)",
                    ));
                }
                Some(Value::Number(value as f64))
            }
            "length" => {
                arity(1, 1)?;
                let length = match argument(0)? {
                    Value::String(value) => value.chars().count(),
                    Value::Array(values) => values.borrow().len(),
                    other => {
                        return Err(runtime_error_at(
                            &span,
                            format!("length is not defined for {}", other.kind_name()),
                        ));
                    }
                };
                Some(Value::Number(length as f64))
            }
            "first" | "last" => {
                arity(2, 2)?;
                let source = text(0)?;
                let count = usize::try_from(require_integer(argument(1)?.clone(), span)?)
                    .map_err(|_| runtime_error_at(&span, "string length cannot be negative"))?;
                let chars: Vec<char> = source.chars().collect();
                let value = if canonical == "first" {
                    chars.iter().take(count).collect::<String>()
                } else {
                    chars
                        .iter()
                        .rev()
                        .take(count)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect()
                };
                Some(Value::String(value))
            }
            "mid" => {
                arity(3, 3)?;
                let source = text(0)?;
                let start = usize::try_from(require_integer(argument(1)?.clone(), span)?).map_err(
                    |_| runtime_error_at(&span, "middle-string start cannot be negative"),
                )?;
                let count = usize::try_from(require_integer(argument(2)?.clone(), span)?).map_err(
                    |_| runtime_error_at(&span, "middle-string length cannot be negative"),
                )?;
                Some(Value::String(
                    source.chars().skip(start).take(count).collect(),
                ))
            }
            "starts_with" | "ends_with" | "contains" => {
                arity(2, 2)?;
                let source = text(0)?;
                let needle = text(1)?;
                let value = match canonical.as_str() {
                    "starts_with" => source.starts_with(needle),
                    "ends_with" => source.ends_with(needle),
                    _ => source.contains(needle),
                };
                Some(Value::Boolean(value))
            }
            "split" => {
                arity(1, 2)?;
                let source = text(0)?;
                let parts: Vec<String> = if arguments.len() == 1 {
                    source.split_whitespace().map(str::to_owned).collect()
                } else {
                    source.split(text(1)?).map(str::to_owned).collect()
                };
                Some(Value::Array(Rc::new(RefCell::new(
                    parts.into_iter().map(Value::String).collect(),
                ))))
            }
            "trim" => {
                arity(1, 1)?;
                Some(Value::String(text(0)?.trim().to_owned()))
            }
            "replace" => {
                arity(3, 3)?;
                Some(Value::String(text(0)?.replace(text(1)?, text(2)?)))
            }
            "to_number" | "number" => {
                arity(1, 1)?;
                Some(Value::Number(text(0)?.trim().parse::<f64>().map_err(
                    |error| {
                        runtime_error_at(&span, format!("cannot convert text to number: {error}"))
                    },
                )?))
            }
            "to_string" => {
                arity(1, 1)?;
                Some(Value::String(argument(0)?.display_text()))
            }
            "type" => {
                arity(1, 1)?;
                Some(Value::String(argument(0)?.kind_name().to_owned()))
            }
            "constant" => {
                arity(1, 1)?;
                let constant = text(0)?;
                match constant {
                    "سطر.جديد" | "newline" => Some(Value::String("\n".to_owned())),
                    _ => None,
                }
            }
            "array" => {
                arity(1, 1)?;
                let length = usize::try_from(require_integer(argument(0)?.clone(), span)?)
                    .map_err(|_| runtime_error_at(&span, "array length cannot be negative"))?;
                Some(Value::Array(Rc::new(RefCell::new(
                    vec![Value::Uninitialized; length],
                ))))
            }
            "multi_array" => {
                arity(1, 1)?;
                let dimensions = match argument(0)? {
                    Value::Array(values) => values
                        .borrow()
                        .iter()
                        .map(|value| {
                            require_integer(value.clone(), span).and_then(|dimension| {
                                usize::try_from(dimension).map_err(|_| {
                                    runtime_error_at(&span, "array dimension cannot be negative")
                                })
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    _ => {
                        return Err(runtime_error_at(
                            &span,
                            "multidimensional array expects an array of dimensions",
                        ));
                    }
                };
                Some(make_multidimensional_array(&dimensions))
            }
            "wait" => {
                arity(1, 1)?;
                let duration = require_integer(argument(0)?.clone(), span)?;
                if duration < 0 {
                    return Err(runtime_error_at(&span, "wait duration cannot be negative"));
                }
                self.host
                    .wait(duration)
                    .map_err(|error| runtime_error_at(&span, error))?;
                Some(Value::Uninitialized)
            }
            "channel" => {
                arity(0, 0)?;
                let id = self
                    .host
                    .create_channel()
                    .map_err(|error| runtime_error_at(&span, error))?;
                Some(Value::Channel(id))
            }
            "draw_line" if arguments.len() == 5 => {
                let coords = arguments[..4]
                    .iter()
                    .cloned()
                    .map(|value| require_integer(value, span))
                    .collect::<Result<Vec<_>, _>>();
                let color = require_integer(arguments[4].clone(), span);
                if let (Ok(coords), Ok(color)) = (coords, color) {
                    self.host
                        .draw_line((coords[0], coords[1]), (coords[2], coords[3]), color)
                        .map_err(|error| runtime_error_at(&span, error))?;
                    Some(Value::Uninitialized)
                } else {
                    Some(
                        self.host
                            .host_call(None, &canonical, arguments.to_vec())
                            .map_err(|error| runtime_error_at(&span, error))?,
                    )
                }
            }
            "file_create" | "file_open" | "file_append" | "file_close" | "file_eof"
            | "clear_screen" | "clear_text" | "set_cursor" | "draw_point" | "draw_line"
            | "draw_rectangle" | "draw_circle" | "load_sprite" | "draw_sprite" | "hide_sprite"
            | "show_sprite" | "sprite_left" | "sprite_right" | "sprite_top" | "sprite_bottom"
            | "sprite_width" | "sprite_height" => Some(
                self.host
                    .host_call(None, &canonical, arguments.to_vec())
                    .map_err(|error| runtime_error_at(&span, error))?,
            ),
            _ => None,
        };
        Ok(result)
    }
}

fn canonical_builtin(name: &str) -> String {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "جا" => "sin",
        "جتا" => "cos",
        "ظا" => "tan",
        "معكوس.جا" => "asin",
        "معكوس.جتا" => "acos",
        "معكوس.ظا" => "atan",
        "جذر" => "sqrt",
        "لو" => "log10",
        "لو.ه" => "ln",
        "تقريب" => "round",
        "باقي.قسمة" => "remainder",
        "عشوائي" => "random",
        "طول" | "عدد" => "length",
        "أول" => "first",
        "آخر" => "last",
        "وسط" => "mid",
        "يبدأ" => "starts_with",
        "ينتهي" => "ends_with",
        "يحتوي" => "contains",
        "تفصيص" => "split",
        "تقليم" => "trim",
        "تبديل" => "replace",
        "رقم" | "كعدد" => "to_number",
        "حرف" => "character",
        "كنص" => "to_string",
        "نوع" => "type",
        "الثابت" => "constant",
        "مصفوفة" => "array",
        "مصفوفة.متعددة" => "multi_array",
        "انتظر" => "wait",
        "قناة" => "channel",
        "انشيء.ملف" => "file_create",
        "افتح.ملف" => "file_open",
        "الحق.بملف" => "file_append",
        "اغلق.ملف" => "file_close",
        "نهاية.الملف" => "file_eof",
        "امسح.الشاشة" => "clear_screen",
        "امسح.الكتابة" => "clear_text",
        "حدد.مكان.المؤشر" => "set_cursor",
        "ارسم.نقطة" => "draw_point",
        "ارسم.خط" => "draw_line",
        "ارسم.مستطيل" => "draw_rectangle",
        "ارسم.دائرة" => "draw_circle",
        "حمل.طيف" => "load_sprite",
        "ارسم.طيف" => "draw_sprite",
        "اخف.طيف" => "hide_sprite",
        "اظهر.طيف" => "show_sprite",
        "يمين.الطيف" => "sprite_right",
        "يسار.الطيف" => "sprite_left",
        "قمة.الطيف" => "sprite_top",
        "قاع.الطيف" => "sprite_bottom",
        "عرض.الطيف" => "sprite_width",
        "ارتفاع.الطيف" => "sprite_height",
        "sqrt" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "log10" | "ln" | "round"
        | "remainder" | "random" | "length" | "first" | "last" | "mid" | "starts_with"
        | "ends_with" | "contains" | "split" | "trim" | "replace" | "to_number" | "number"
        | "to_string" | "character" | "type" | "constant" | "array" | "multi_array" | "wait"
        | "channel" | "file_create" | "file_open" | "file_append" | "file_close" | "file_eof"
        | "clear_screen" | "clear_text" | "set_cursor" | "draw_point" | "draw_line"
        | "draw_rectangle" | "draw_circle" | "load_sprite" | "draw_sprite" | "hide_sprite"
        | "show_sprite" | "sprite_left" | "sprite_right" | "sprite_top" | "sprite_bottom"
        | "sprite_width" | "sprite_height" => lower.as_str(),
        _ => return name.to_owned(),
    }
    .to_owned()
}

fn require_integer(value: Value, span: Span) -> Result<i64, RuntimeError> {
    match value {
        Value::Number(number)
            if number.is_finite()
                && number.fract() == 0.0
                && number >= i64::MIN as f64
                && number <= i64::MAX as f64 =>
        {
            Ok(number as i64)
        }
        other => Err(runtime_error_at(
            &span,
            format!("expected an integer, got {}", other.kind_name()),
        )),
    }
}

fn coerce_number(value: Value) -> Result<Value, String> {
    match value {
        Value::Number(_) => Ok(value),
        Value::String(text) => text
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|error| format!("input is not numeric: {error}")),
        other => Err(format!("cannot convert {} to a number", other.kind_name())),
    }
}

fn make_multidimensional_array(dimensions: &[usize]) -> Value {
    let length = dimensions.first().copied().unwrap_or(0);
    let items = (0..length)
        .map(|_| {
            if dimensions.len() <= 1 {
                Value::Uninitialized
            } else {
                make_multidimensional_array(&dimensions[1..])
            }
        })
        .collect();
    Value::Array(Rc::new(RefCell::new(items)))
}

fn is_type_name(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "integer"
            | "number"
            | "real"
            | "string"
            | "text"
            | "boolean"
            | "bool"
            | "array"
            | "dictionary"
            | "object"
            | "عدد"
            | "نص"
            | "منطقي"
            | "مصفوفة"
            | "قاموس"
            | "كائن"
    )
}

fn apply_binary(
    left: Value,
    operator: syntax::BinaryOp,
    right: Value,
    span: Span,
    classes: &HashMap<String, syntax::ClassDef>,
) -> Result<Value, RuntimeError> {
    use syntax::BinaryOp as B;
    let number_pair = || match (&left, &right) {
        (Value::Number(a), Value::Number(b)) => Ok((*a, *b)),
        _ => Err(runtime_error_at(
            &span,
            "arithmetic operator expects numbers",
        )),
    };
    let result = match operator {
        B::Add => match (&left, &right) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::String(a), Value::String(b)) => Value::String(format!("{a}{b}")),
            _ => {
                return Err(runtime_error_at(
                    &span,
                    "addition expects two numbers or two strings",
                ));
            }
        },
        B::Subtract => {
            let (a, b) = number_pair()?;
            Value::Number(a - b)
        }
        B::Multiply => {
            let (a, b) = number_pair()?;
            Value::Number(a * b)
        }
        B::Divide => {
            let (a, b) = number_pair()?;
            if b == 0.0 {
                return Err(runtime_error_at(&span, "division by zero"));
            }
            Value::Number(a / b)
        }
        B::Equal => Value::Boolean(left == right),
        B::NotEqual => Value::Boolean(left != right),
        B::Less | B::LessEqual | B::Greater | B::GreaterEqual => {
            let ordering = match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
                (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
                _ => {
                    return Err(runtime_error_at(
                        &span,
                        "comparison expects two numbers or two strings",
                    ));
                }
            }
            .ok_or_else(|| runtime_error_at(&span, "values cannot be ordered"))?;
            Value::Boolean(match operator {
                B::Less => ordering.is_lt(),
                B::LessEqual => !ordering.is_gt(),
                B::Greater => ordering.is_gt(),
                B::GreaterEqual => !ordering.is_lt(),
                _ => unreachable!(),
            })
        }
        B::IsA => Value::Boolean(value_is_a(&left, &right, classes)),
        B::And | B::Or | B::Nor => {
            return Err(runtime_error_at(
                &span,
                "logical operators require boolean evaluation",
            ));
        }
    };
    Ok(result)
}

fn value_is_a(
    value: &Value,
    type_value: &Value,
    classes: &HashMap<String, syntax::ClassDef>,
) -> bool {
    let type_name = match type_value {
        Value::TypeName(name) | Value::Class(name) | Value::String(name) => name.to_lowercase(),
        _ => return false,
    };
    let actual = match value {
        Value::Number(number) if number.fract() == 0.0 => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Boolean(_) => "boolean",
        Value::Array(_) => "array",
        Value::Dictionary(_) => "dictionary",
        Value::Object(object) => {
            return class_is_a(
                &object.borrow().class_name,
                &type_name,
                classes,
                &mut HashSet::new(),
            );
        }
        Value::Channel(_) => "channel",
        Value::File(_) => "file",
        Value::Class(_) => "class",
        Value::TypeName(_) => "type",
        Value::Uninitialized => "uninitialized",
    };
    actual == type_name
        || (actual == "integer" && type_name == "number")
        || (actual == "string" && type_name == "text")
        || (actual == "boolean" && type_name == "bool")
}

fn class_is_a(
    class_name: &str,
    target: &str,
    classes: &HashMap<String, syntax::ClassDef>,
    seen: &mut HashSet<String>,
) -> bool {
    if class_name.to_lowercase() == target {
        return true;
    }
    if !seen.insert(class_name.to_owned()) {
        return false;
    }
    classes.get(class_name).is_some_and(|class| {
        class
            .bases
            .iter()
            .any(|base| class_is_a(base, target, classes, seen))
    })
}

fn validate_dictionary_key(value: &Value) -> Result<(), String> {
    validate_dictionary_key_inner(value, &mut HashSet::new())
}

fn validate_dictionary_key_inner(
    value: &Value,
    seen_arrays: &mut HashSet<usize>,
) -> Result<(), String> {
    match value {
        Value::Number(number) if number.is_finite() && number.fract() == 0.0 => Ok(()),
        Value::String(_) => Ok(()),
        Value::Number(_) => Err("dictionary numeric keys must be integers".to_owned()),
        Value::Array(values) => {
            let identity = Rc::as_ptr(values) as usize;
            if !seen_arrays.insert(identity) {
                return Ok(());
            }
            for element in values.borrow().iter() {
                validate_dictionary_key_inner(element, seen_arrays)?;
            }
            seen_arrays.remove(&identity);
            Ok(())
        }
        other => Err(format!(
            "{} values are not supported as dictionary keys",
            other.kind_name()
        )),
    }
}

fn validate_graphics_arguments(
    command: &str,
    arguments: &[Value],
    span: Span,
) -> Result<(), RuntimeError> {
    let (color_index, fill_index) = match command {
        "draw_point" => (Some(2), None),
        "draw_line" => (Some(4), None),
        "draw_rectangle" => (Some(4), Some(5)),
        "draw_circle" => (Some(3), Some(4)),
        _ => return Ok(()),
    };
    if let Some(index) = color_index {
        if let Some(value) = arguments
            .get(index)
            .filter(|value| !matches!(value, Value::Uninitialized))
        {
            let color = require_integer(value.clone(), span)?;
            if !(0..=15).contains(&color) {
                return Err(runtime_error_at(
                    &span,
                    "graphics color must be between 0 and 15",
                ));
            }
        }
    }
    if let Some(index) = fill_index {
        if let Some(value) = arguments
            .get(index)
            .filter(|value| !matches!(value, Value::Uninitialized))
        {
            if !matches!(value, Value::Boolean(_)) {
                return Err(runtime_error_at(
                    &span,
                    "graphics fill option must be boolean",
                ));
            }
        }
    }
    Ok(())
}

fn dictionary_key_equal(left: &Value, right: &Value) -> bool {
    key_equal_inner(left, right, &mut HashSet::new())
}

fn key_equal_inner(left: &Value, right: &Value, seen: &mut HashSet<(usize, usize)>) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            let pair = (Rc::as_ptr(a) as usize, Rc::as_ptr(b) as usize);
            if Rc::ptr_eq(a, b) || !seen.insert(pair) {
                return true;
            }
            let a_values = a.borrow();
            let b_values = b.borrow();
            a_values.len() == b_values.len()
                && a_values
                    .iter()
                    .zip(b_values.iter())
                    .all(|(a, b)| key_equal_inner(a, b, seen))
        }
        _ => false,
    }
}

fn runtime_error(message: impl Into<String>) -> RuntimeError {
    RuntimeError {
        message: message.into(),
    }
}
fn runtime_error_at(span: &Span, message: impl Into<String>) -> RuntimeError {
    runtime_error(format!("{}:{}: {}", span.line, span.column, message.into()))
}
