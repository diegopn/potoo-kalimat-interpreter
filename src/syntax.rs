use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.span.line, self.span.column, self.message
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedProgram {
    pub main: Vec<Stmt>,
    pub unit_globals: Vec<Stmt>,
    pub procedures: HashMap<String, Procedure>,
    pub functions: HashMap<String, Procedure>,
    pub classes: HashMap<String, ClassDef>,
    pub methods: Vec<MethodDef>,
    pub units: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct Procedure {
    pub parameters: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ClassDef {
    pub fields: Vec<String>,
    pub bases: Vec<String>,
    pub declared_methods: Vec<MethodSignature>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub(crate) struct MethodSignature {
    pub name: String,
    pub parameters: Vec<String>,
    pub returns_value: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct MethodDef {
    pub class_name: String,
    pub receiver: String,
    pub name: String,
    pub parameters: Vec<String>,
    pub body: Vec<Stmt>,
    pub returns_value: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub(crate) struct Stmt {
    pub span: Span,
    pub kind: StmtKind,
}

#[derive(Clone, Debug)]
pub(crate) enum StmtKind {
    Assign {
        target: Target,
        value: Expr,
    },
    Read {
        prompt: Option<Expr>,
        targets: Vec<(Target, bool)>,
    },
    ReadFile {
        file: Expr,
        targets: Vec<Target>,
    },
    Print {
        values: Vec<Expr>,
        newline: bool,
    },
    PrintFile {
        file: Expr,
        values: Vec<Expr>,
        newline: bool,
    },
    Graphics {
        command: String,
        arguments: Vec<Option<Expr>>,
    },
    If {
        branches: Vec<(Expr, Vec<Stmt>)>,
        otherwise: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Loop {
        body: Vec<Stmt>,
    },
    For {
        variable: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        descending: bool,
        body: Vec<Stmt>,
    },
    Label(String),
    Goto(String),
    Return(Option<Expr>),
    TailCall {
        name: String,
        arguments: Vec<Expr>,
    },
    Launch {
        name: String,
        arguments: Vec<Expr>,
    },
    Send {
        channel: Expr,
        value: Option<Expr>,
    },
    Receive {
        channel: Expr,
        target: Option<Target>,
    },
    Select(Vec<SelectBranch>),
    RegisterEvent {
        event: String,
        procedure: String,
    },
    Shared(String),
    UseUnit(String),
    Expression(Expr),
}

#[derive(Clone, Debug)]
pub(crate) struct SelectBranch {
    pub operation: SelectOperation,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub(crate) enum SelectOperation {
    Send {
        channel: Expr,
        value: Option<Expr>,
    },
    Receive {
        channel: Expr,
        target: Option<Target>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct Target {
    pub span: Span,
    pub kind: TargetKind,
}

#[derive(Clone, Debug)]
pub(crate) enum TargetKind {
    Variable(String),
    Index { object: Expr, indices: Vec<Expr> },
    Field { object: Expr, name: String },
}

#[derive(Clone, Debug)]
pub(crate) struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Clone, Debug)]
pub(crate) enum ExprKind {
    Number(f64),
    String(String),
    Boolean(bool),
    Variable(String),
    Array(Vec<Expr>),
    Dictionary(Vec<(Expr, Expr)>),
    Unary {
        operator: UnaryOp,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        name: String,
        arguments: Vec<Expr>,
    },
    Index {
        object: Box<Expr>,
        indices: Vec<Expr>,
    },
    Field {
        object: Box<Expr>,
        name: String,
    },
    MethodCall {
        object: Box<Expr>,
        name: String,
        arguments: Vec<Expr>,
    },
    NewObject(String),
    NewWidget(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    Negate,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Nor,
    IsA,
}

#[derive(Clone, Debug)]
struct SourceLine {
    number: usize,
    text: String,
}

pub(crate) fn parse(source: &str) -> Result<ParsedProgram, Vec<Diagnostic>> {
    Parser::new(source).parse_program()
}

struct Parser {
    lines: Vec<SourceLine>,
    at: usize,
    main: Vec<Stmt>,
    procedures: HashMap<String, Procedure>,
    functions: HashMap<String, Procedure>,
    classes: HashMap<String, ClassDef>,
    methods: Vec<MethodDef>,
    units: Vec<String>,
    unit_globals: Vec<Stmt>,
    in_unit: bool,
}

impl Parser {
    fn new(source: &str) -> Self {
        let lines = source
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let trimmed = line.trim();
                (!trimmed.is_empty()).then(|| SourceLine {
                    number: index + 1,
                    text: trimmed.to_owned(),
                })
            })
            .collect();
        Self {
            lines,
            at: 0,
            main: Vec::new(),
            procedures: HashMap::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            methods: Vec::new(),
            units: Vec::new(),
            unit_globals: Vec::new(),
            in_unit: false,
        }
    }

    fn parse_program(mut self) -> Result<ParsedProgram, Vec<Diagnostic>> {
        while let Some(line) = self.current().cloned() {
            let lower = line.text.to_lowercase();
            if starts_any(&lower, &["إجراء ", "proc "]) {
                let (name, procedure) = self.parse_procedure(false)?;
                if self.functions.contains_key(&name) {
                    return Err(vec![diagnostic(
                        line.number,
                        1,
                        format!("duplicate definition `{name}`"),
                    )]);
                }
                insert_unique(&mut self.procedures, name, procedure, line.number)?;
            } else if starts_any(&lower, &["دالة ", "func "]) {
                let (name, function) = self.parse_procedure(true)?;
                if self.procedures.contains_key(&name) {
                    return Err(vec![diagnostic(
                        line.number,
                        1,
                        format!("duplicate definition `{name}`"),
                    )]);
                }
                insert_unique(&mut self.functions, name, function, line.number)?;
            } else if starts_any(&lower, &["فصيلة ", "class "]) {
                let (name, class) = self.parse_class()?;
                insert_unique(&mut self.classes, name, class, line.number)?;
            } else if starts_any(&lower, &["استجابة ", "رد ", "response ", "reply "]) {
                let method = self.parse_method()?;
                if self.methods.iter().any(|existing| {
                    existing.class_name == method.class_name && existing.name == method.name
                }) {
                    return Err(vec![diagnostic(
                        method.span.line,
                        1,
                        format!("duplicate method `{}`", method.name),
                    )]);
                }
                self.methods.push(method);
            } else if starts_any(&lower, &["وحدة ", "unit "]) {
                self.in_unit = true;
                self.at += 1;
            } else {
                let statement = self.parse_statement()?;
                if let StmtKind::UseUnit(name) = &statement.kind {
                    self.units.push(name.clone());
                } else if self.in_unit {
                    self.unit_globals.push(statement);
                } else {
                    self.main.push(statement);
                }
            }
        }
        Ok(ParsedProgram {
            main: self.main,
            unit_globals: self.unit_globals,
            procedures: self.procedures,
            functions: self.functions,
            classes: self.classes,
            methods: self.methods,
            units: self.units,
        })
    }

    fn current(&self) -> Option<&SourceLine> {
        self.lines.get(self.at)
    }

    fn line_span(line: usize) -> Span {
        Span { line, column: 1 }
    }

    fn parse_procedure(
        &mut self,
        returns_value: bool,
    ) -> Result<(String, Procedure), Vec<Diagnostic>> {
        let source = self.current().cloned().expect("procedure header exists");
        let keyword = if returns_value {
            "دالة"
        } else {
            "إجراء"
        };
        let english = if returns_value { "func" } else { "proc" };
        let header = strip_keyword(&source.text, &[keyword, english]);
        let (name, parameters) = parse_header_call(source.number, header)?;
        self.at += 1;
        let body = self.parse_block(&["نهاية", "end"])?;
        self.consume_terminator(&["نهاية", "end"], source.number)?;
        Ok((
            name,
            Procedure {
                parameters,
                body,
                span: Self::line_span(source.number),
            },
        ))
    }

    fn parse_class(&mut self) -> Result<(String, ClassDef), Vec<Diagnostic>> {
        let source = self.current().cloned().expect("class header exists");
        let header = strip_keyword(&source.text, &["فصيلة", "class"])
            .trim_end_matches(':')
            .trim();
        if header.is_empty() {
            return Err(vec![diagnostic(source.number, 1, "expected class name")]);
        }
        let mut class = ClassDef {
            span: Self::line_span(source.number),
            ..ClassDef::default()
        };
        self.at += 1;
        while let Some(line) = self.current().cloned() {
            let text = line.text.trim();
            if matches_word(text, &["نهاية", "end"]) {
                break;
            }
            if starts_any(&text.to_lowercase(), &["له ", "has "]) {
                let fields = strip_keyword(text, &["له", "has"]);
                class.fields.extend(
                    split_top_level(fields, ',')
                        .into_iter()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                );
            } else if starts_any(&text.to_lowercase(), &["مبني على ", "built on "]) {
                let bases = strip_keyword(text, &["مبني على", "built on"]);
                class.bases.extend(
                    split_top_level(bases, ',')
                        .into_iter()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                );
            } else if starts_any(
                &text.to_lowercase(),
                &["يستجيب ل", "responds to", "يرد على", "replies to"],
            ) {
                let returns = starts_any(&text.to_lowercase(), &["يرد على", "replies to"]);
                let tail =
                    strip_keyword(text, &["يستجيب ل", "responds to", "يرد على", "replies to"])
                        .trim()
                        .trim_start_matches(':')
                        .trim();
                for signature in split_top_level(tail, ',') {
                    let (name, parameters) = parse_header_call(line.number, signature.trim())?;
                    class.declared_methods.push(MethodSignature {
                        name,
                        parameters,
                        returns_value: returns,
                    });
                }
            } else {
                return Err(vec![diagnostic(line.number, 1, "unsupported class member")]);
            }
            self.at += 1;
        }
        self.consume_terminator(&["نهاية", "end"], source.number)?;
        Ok((header.to_owned(), class))
    }

    fn parse_method(&mut self) -> Result<MethodDef, Vec<Diagnostic>> {
        let source = self.current().cloned().expect("method header exists");
        let returns_value = starts_any(&source.text.to_lowercase(), &["رد ", "reply "]);
        let tail = strip_keyword(&source.text, &["استجابة", "رد", "response", "reply"]);
        let (owner_and_receiver, method) = split_word_top_level(
            tail,
            if starts_any(&source.text.to_lowercase(), &["رد ", "reply "]) {
                "على"
            } else {
                "ل"
            },
        )
        .or_else(|| split_word_top_level(tail, if returns_value { "to" } else { "for" }))
        .ok_or_else(|| {
            vec![diagnostic(
                source.number,
                1,
                "expected method receiver and method name",
            )]
        })?;
        let mut owner_words = owner_and_receiver.split_whitespace();
        let class_name = owner_words.next().unwrap_or_default().to_owned();
        let receiver = owner_words.next().unwrap_or_default().to_owned();
        if class_name.is_empty() || receiver.is_empty() {
            return Err(vec![diagnostic(
                source.number,
                1,
                "expected class name and explicit receiver",
            )]);
        }
        let (name, parameters) = parse_header_call(
            source.number,
            method
                .trim()
                .trim_start_matches(':')
                .trim()
                .trim_end_matches(':'),
        )?;
        self.at += 1;
        let body = self.parse_block(&["نهاية", "end"])?;
        self.consume_terminator(&["نهاية", "end"], source.number)?;
        Ok(MethodDef {
            class_name,
            receiver,
            name,
            parameters,
            body,
            returns_value,
            span: Self::line_span(source.number),
        })
    }

    fn parse_block(&mut self, terminators: &[&str]) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        while let Some(line) = self.current() {
            if matches_word(&line.text, terminators) {
                return Ok(statements);
            }
            statements.push(self.parse_statement()?);
        }
        let line = self.lines.last().map_or(1, |line| line.number);
        Err(vec![diagnostic(
            line,
            1,
            format!("missing `{}`", terminators[0]),
        )])
    }

    fn consume_terminator(
        &mut self,
        terminators: &[&str],
        fallback_line: usize,
    ) -> Result<(), Vec<Diagnostic>> {
        let Some(line) = self.current() else {
            return Err(vec![diagnostic(
                fallback_line,
                1,
                format!("missing `{}`", terminators[0]),
            )]);
        };
        if !matches_word(&line.text, terminators) {
            return Err(vec![diagnostic(
                line.number,
                1,
                format!("expected `{}`", terminators[0]),
            )]);
        }
        self.at += 1;
        Ok(())
    }

    fn parse_statement(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let source = self.current().cloned().expect("statement exists");
        let lower = source.text.to_lowercase();
        let span = Self::line_span(source.number);
        if starts_any(&lower, &["إذا ", "if "]) {
            return self.parse_if();
        }
        if starts_any(&lower, &["كرر مادام ", "while "]) {
            let condition_source = strip_keyword(&source.text, &["كرر مادام", "while"]);
            let condition =
                parse_expression(source.number, condition_source.trim().trim_end_matches(':'))?;
            self.at += 1;
            let body = self.parse_block(&["تابع", "loop"])?;
            self.consume_terminator(&["تابع", "loop"], source.number)?;
            return Ok(Stmt {
                span,
                kind: StmtKind::While { condition, body },
            });
        }
        if starts_any(&lower, &["كرر:", "loop:"]) {
            self.at += 1;
            let body = self.parse_block(&["تابع", "loop"])?;
            self.consume_terminator(&["تابع", "loop"], source.number)?;
            return Ok(Stmt {
                span,
                kind: StmtKind::Loop { body },
            });
        }
        if starts_any(&lower, &["لكل ", "for "]) {
            return self.parse_for();
        }
        if starts_any(&lower, &["تخير:", "select:"]) {
            return self.parse_select();
        }
        if starts_any(&lower, &["ارسم.", "draw."]) {
            return self.parse_graphics();
        }
        if starts_any(&lower, &["اطبع في ", "print in "]) {
            let tail = strip_keyword(&source.text, &["اطبع في", "print in"]);
            let (file, values) = split_once_top_level(tail, ':')
                .ok_or_else(|| vec![diagnostic(source.number, 1, "expected `:` after file")])?;
            let file = parse_expression(source.number, file.trim())?;
            let (values, newline) = strip_ellipsis(values.trim());
            let values = parse_expression_list(source.number, values)?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::PrintFile {
                    file,
                    values,
                    newline,
                },
            });
        }
        if starts_any(&lower, &["اطبع ", "print "]) || lower == "اطبع" || lower == "print" {
            let tail = strip_keyword(&source.text, &["اطبع", "print"]);
            let (tail, newline) = strip_ellipsis(tail.trim());
            let values = parse_expression_list(source.number, tail)?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Print { values, newline },
            });
        }
        if starts_any(&lower, &["اقرأ من ", "read from "]) {
            let tail = strip_keyword(&source.text, &["اقرأ من", "read from"]);
            let (file, destinations) = split_once_top_level(tail, ':')
                .ok_or_else(|| vec![diagnostic(source.number, 1, "expected `:` after file")])?;
            let file = parse_expression(source.number, file.trim())?;
            let targets = split_top_level(destinations, ',')
                .into_iter()
                .map(|item| parse_target(source.number, item.trim()))
                .collect::<Result<Vec<_>, _>>()?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::ReadFile { file, targets },
            });
        }
        if starts_any(&lower, &["اقرأ ", "read "]) || lower == "اقرأ" || lower == "read" {
            let tail = strip_keyword(&source.text, &["اقرأ", "read"]);
            let parts = split_top_level(tail.trim(), ',');
            let mut prompt = None;
            let mut targets = Vec::new();
            let mut index = 0;
            if let Some(first) = parts.first().filter(|part| starts_with_quote(part.trim())) {
                prompt = Some(parse_expression(source.number, first.trim())?);
                index = 1;
            }
            for destination in &parts[index..] {
                let destination = destination.trim();
                let numeric = destination.starts_with('#');
                targets.push((
                    parse_target(source.number, destination.trim_start_matches('#').trim())?,
                    numeric,
                ));
            }
            if targets.is_empty() {
                return Err(vec![diagnostic(
                    source.number,
                    1,
                    "read expects at least one destination",
                )]);
            }
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Read { prompt, targets },
            });
        }
        if starts_any(&lower, &["شغل ", "launch "]) {
            let call_text = strip_keyword(&source.text, &["شغل", "launch"]);
            let (name, arguments) = parse_call_expression(source.number, call_text.trim())?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Launch { name, arguments },
            });
        }
        if starts_any(&lower, &["ارسل ", "send "]) {
            let tail = strip_keyword(&source.text, &["ارسل", "send"]);
            let (value, channel) = split_word_top_level(
                tail,
                if lower.starts_with("send ") {
                    "to"
                } else {
                    "إلى"
                },
            )
            .ok_or_else(|| vec![diagnostic(source.number, 1, "expected channel destination")])?;
            let value = value.trim();
            let value = if starts_any(&value.to_lowercase(), &["إشارة", "signal"]) {
                None
            } else {
                Some(parse_expression(source.number, value)?)
            };
            let channel = parse_expression(source.number, channel.trim().trim_end_matches(':'))?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Send { channel, value },
            });
        }
        if starts_any(&lower, &["تسلم ", "receive "]) {
            let tail = strip_keyword(&source.text, &["تسلم", "receive"]);
            let (destination, channel) = split_word_top_level(
                tail,
                if lower.starts_with("receive ") {
                    "from"
                } else {
                    "من"
                },
            )
            .ok_or_else(|| vec![diagnostic(source.number, 1, "expected channel source")])?;
            let destination = destination.trim();
            let target = if starts_any(&destination.to_lowercase(), &["إشارة", "signal"]) {
                None
            } else {
                Some(parse_target(source.number, destination)?)
            };
            let channel = parse_expression(source.number, channel.trim().trim_end_matches(':'))?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Receive { channel, target },
            });
        }
        if starts_any(&lower, &["عند حادثة ", "on event "]) {
            let tail = strip_keyword(&source.text, &["عند حادثة", "on event"]);
            let (event, procedure) = split_word_top_level(
                tail,
                if lower.starts_with("on event ") {
                    "do"
                } else {
                    "نفذ"
                },
            )
            .ok_or_else(|| {
                vec![diagnostic(
                    source.number,
                    1,
                    "expected event handler procedure",
                )]
            })?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::RegisterEvent {
                    event: event.trim().to_owned(),
                    procedure: procedure.trim().to_owned(),
                },
            });
        }
        if starts_any(&lower, &["اذهب إلى ", "goto "]) {
            let name = strip_keyword(&source.text, &["اذهب إلى", "goto"])
                .trim()
                .to_owned();
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Goto(name),
            });
        }
        if starts_any(&lower, &["علامة ", "label "]) {
            let name = strip_keyword(&source.text, &["علامة", "label"])
                .trim()
                .trim_end_matches(':')
                .to_owned();
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Label(name),
            });
        }
        if starts_any(&lower, &["وكل إلى ", "delegate to "]) {
            let call_text = strip_keyword(&source.text, &["وكل إلى", "delegate to"]);
            let (name, arguments) = parse_call_expression(source.number, call_text.trim())?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::TailCall { name, arguments },
            });
        }
        if starts_any(&lower, &["ارجع ب", "return "]) {
            let tail = strip_keyword(&source.text, &["ارجع ب", "return"])
                .trim_start_matches(':')
                .trim();
            let value = if tail.is_empty() {
                None
            } else {
                Some(parse_expression(source.number, tail)?)
            };
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Return(value),
            });
        }
        if starts_any(&lower, &["مشترك ", "shared "]) {
            let name = strip_keyword(&source.text, &["مشترك", "shared"])
                .trim()
                .to_owned();
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Shared(name),
            });
        }
        if starts_any(&lower, &["باستخدام ", "using "]) {
            let tail = strip_keyword(&source.text, &["باستخدام", "using"]).trim();
            let name = if starts_with_quote(tail) {
                parse_string_literal(source.number, tail)?
            } else {
                tail.to_owned()
            };
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::UseUnit(name),
            });
        }
        if starts_any(&lower, &["انتظر(", "wait("]) {
            let expression = parse_expression(source.number, &source.text)?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Expression(expression),
            });
        }
        let words: Vec<_> = source.text.split_whitespace().collect();
        if words.len() == 2
            && matches!(
                words[1].to_lowercase().as_str(),
                "مشترك" | "مشتركة" | "shared"
            )
        {
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Shared(words[0].to_owned()),
            });
        }
        if let Some(equals) = find_assignment(source.text.as_str()) {
            let target = parse_target(source.number, source.text[..equals].trim())?;
            let value = parse_expression(source.number, source.text[equals + 1..].trim())?;
            self.at += 1;
            return Ok(Stmt {
                span,
                kind: StmtKind::Assign { target, value },
            });
        }
        let expression = parse_expression(source.number, &source.text)?;
        self.at += 1;
        Ok(Stmt {
            span,
            kind: StmtKind::Expression(expression),
        })
    }

    fn parse_graphics(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let source = self.current().cloned().expect("graphics statement exists");
        let lower = source.text.to_lowercase();
        if starts_any(&lower, &["ارسم.طيف ", "draw.sprite "]) {
            let tail = strip_keyword(&source.text, &["ارسم.طيف", "draw.sprite"]);
            let (sprite, location) = split_word_top_level(
                tail,
                if lower.starts_with("draw.sprite ") {
                    "at"
                } else {
                    "في"
                },
            )
            .ok_or_else(|| vec![diagnostic(source.number, 1, "expected sprite location")])?;
            let location = location.trim();
            let open = location
                .find('(')
                .ok_or_else(|| vec![diagnostic(source.number, 1, "expected sprite coordinates")])?;
            let close = matching_close(location, open).ok_or_else(|| {
                vec![diagnostic(
                    source.number,
                    open + 1,
                    "expected `)` after sprite coordinates",
                )]
            })?;
            if !location[..open].trim().is_empty() || !location[close + 1..].trim().is_empty() {
                return Err(vec![diagnostic(
                    source.number,
                    1,
                    "unexpected text in sprite placement",
                )]);
            }
            let mut arguments = vec![Some(parse_expression(source.number, sprite.trim())?)];
            let coordinates = parse_expression_list(source.number, &location[open + 1..close])?;
            if coordinates.len() != 2 {
                return Err(vec![diagnostic(
                    source.number,
                    open + 1,
                    "sprite location expects two coordinates",
                )]);
            }
            arguments.extend(coordinates.into_iter().map(Some));
            self.at += 1;
            return Ok(Stmt {
                span: Self::line_span(source.number),
                kind: StmtKind::Graphics {
                    command: "draw_sprite".to_owned(),
                    arguments,
                },
            });
        }
        let open = source.text.find('(').ok_or_else(|| {
            vec![diagnostic(
                source.number,
                1,
                "expected `(` in graphics command",
            )]
        })?;
        let command = source.text[..open].trim().to_owned();
        let close = matching_close(&source.text, open).ok_or_else(|| {
            vec![diagnostic(
                source.number,
                open + 1,
                "expected `)` in graphics command",
            )]
        })?;
        let mut arguments = parse_expression_list(source.number, &source.text[open + 1..close])?
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        let mut tail = source.text[close + 1..].trim();
        let lower_command = command.to_lowercase();
        if lower_command.contains("خط")
            || lower_command.contains("line")
            || lower_command.contains("مستطيل")
            || lower_command.contains("rectangle")
        {
            tail = tail
                .strip_prefix('-')
                .ok_or_else(|| {
                    vec![diagnostic(
                        source.number,
                        close + 2,
                        "expected second point after `-`",
                    )]
                })?
                .trim();
            let second_open = tail.find('(').ok_or_else(|| {
                vec![diagnostic(
                    source.number,
                    close + 2,
                    "expected second point",
                )]
            })?;
            if !tail[..second_open].trim().is_empty() {
                return Err(vec![diagnostic(
                    source.number,
                    close + 2,
                    "unexpected text before second point",
                )]);
            }
            let second_close = matching_close(tail, second_open).ok_or_else(|| {
                vec![diagnostic(
                    source.number,
                    close + second_open + 2,
                    "expected `)` after second point",
                )]
            })?;
            arguments.extend(
                parse_expression_list(source.number, &tail[second_open + 1..second_close])?
                    .into_iter()
                    .map(Some),
            );
            tail = tail[second_close + 1..].trim();
        }
        if !tail.is_empty() {
            let tail = tail.strip_prefix(',').ok_or_else(|| {
                vec![diagnostic(
                    source.number,
                    source.text.len() - tail.len() + 1,
                    "expected comma before graphics options",
                )]
            })?;
            for option in split_top_level(tail, ',') {
                let option = option.trim();
                arguments.push(if option.is_empty() {
                    None
                } else {
                    Some(parse_expression(source.number, option)?)
                });
            }
        }
        self.at += 1;
        Ok(Stmt {
            span: Self::line_span(source.number),
            kind: StmtKind::Graphics { command, arguments },
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let source = self.current().cloned().expect("if header exists");
        let mut branches = Vec::new();
        let mut otherwise = Vec::new();
        let condition = parse_expression(
            source.number,
            strip_keyword(&source.text, &["إذا", "if"])
                .trim()
                .trim_end_matches(':'),
        )?;
        self.at += 1;
        branches.push((condition, self.parse_if_arm()?));
        while let Some(line) = self.current() {
            let lower = line.text.to_lowercase();
            if starts_any(&lower, &["وإلا إذا ", "else if "]) {
                let line = line.clone();
                let condition = parse_expression(
                    line.number,
                    strip_keyword(&line.text, &["وإلا إذا", "else if"])
                        .trim()
                        .trim_end_matches(':'),
                )?;
                self.at += 1;
                branches.push((condition, self.parse_if_arm()?));
            } else if starts_any(&lower, &["وإلا", "else"]) {
                self.at += 1;
                otherwise = self.parse_if_arm()?;
                break;
            } else {
                break;
            }
        }
        self.consume_terminator(&["تم", "done"], source.number)?;
        Ok(Stmt {
            span: Self::line_span(source.number),
            kind: StmtKind::If {
                branches,
                otherwise,
            },
        })
    }

    fn parse_if_arm(&mut self) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        while let Some(line) = self.current() {
            let lower = line.text.to_lowercase();
            if matches_word(&line.text, &["تم", "done"]) || starts_any(&lower, &["وإلا", "else"])
            {
                break;
            }
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_for(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let source = self.current().cloned().expect("for header exists");
        let header = strip_keyword(&source.text, &["لكل", "for"])
            .trim()
            .trim_end_matches(':')
            .trim();
        let (variable, tail) = split_first_word(header);
        if variable.is_empty() {
            return Err(vec![diagnostic(source.number, 1, "expected loop variable")]);
        }
        let lower = tail.to_lowercase();
        let descending = lower.contains("نزولا إلى") || lower.contains("down to");
        let (start_text, bounds_tail) = if descending {
            split_word_top_level(
                tail,
                if lower.contains("نزولا إلى") {
                    "نزولا إلى"
                } else {
                    "down to"
                },
            )
            .map(|pair| pair)
            .ok_or_else(|| {
                vec![diagnostic(
                    source.number,
                    1,
                    "expected descending loop bound",
                )]
            })?
        } else {
            let tail = strip_keyword(tail, &["من", "from"]);
            split_word_top_level(
                tail,
                if tail.contains("إلى") {
                    "إلى"
                } else {
                    "to"
                },
            )
            .ok_or_else(|| vec![diagnostic(source.number, 1, "expected loop range")])?
        };
        let start_text =
            if descending && starts_any(&start_text.trim().to_lowercase(), &["من ", "from "]) {
                strip_keyword(start_text.trim(), &["من", "from"])
            } else {
                start_text
            };
        let (end_text, step_text) = split_word_top_level(
            bounds_tail,
            if bounds_tail.contains("بخطوة") {
                "بخطوة"
            } else {
                "step"
            },
        )
        .map(|(end, step)| (end, Some(step)))
        .unwrap_or((bounds_tail, None));
        let start = parse_expression(source.number, start_text.trim())?;
        let end = parse_expression(source.number, end_text.trim())?;
        let step = step_text
            .map(|text| parse_expression(source.number, text.trim()))
            .transpose()?;
        self.at += 1;
        let body = self.parse_block(&["تابع", "loop"])?;
        self.consume_terminator(&["تابع", "loop"], source.number)?;
        Ok(Stmt {
            span: Self::line_span(source.number),
            kind: StmtKind::For {
                variable: variable.to_owned(),
                start,
                end,
                step,
                descending,
                body,
            },
        })
    }

    fn parse_select(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let source = self.current().cloned().expect("select header exists");
        self.at += 1;
        let mut branches = Vec::new();
        loop {
            let Some(header) = self.current().cloned() else {
                return Err(vec![diagnostic(
                    source.number,
                    1,
                    "missing `تم` for select",
                )]);
            };
            if matches_word(&header.text, &["تم", "done"]) {
                self.at += 1;
                break;
            }
            let operation_text = strip_keyword(&header.text, &["أو", "or"])
                .trim()
                .trim_end_matches(':')
                .trim();
            let lower = operation_text.to_lowercase();
            let operation = if starts_any(&lower, &["ارسل ", "send "]) {
                let tail = strip_keyword(operation_text, &["ارسل", "send"]);
                let (value, channel) = split_word_top_level(
                    tail,
                    if lower.starts_with("send ") {
                        "to"
                    } else {
                        "إلى"
                    },
                )
                .ok_or_else(|| {
                    vec![diagnostic(
                        header.number,
                        1,
                        "expected select send destination",
                    )]
                })?;
                let value = value.trim();
                let value = if starts_any(&value.to_lowercase(), &["إشارة", "signal"]) {
                    None
                } else {
                    Some(parse_expression(header.number, value)?)
                };
                SelectOperation::Send {
                    channel: parse_expression(header.number, strip_branch_marker(channel.trim()))?,
                    value,
                }
            } else if starts_any(&lower, &["تسلم ", "receive "]) {
                let tail = strip_keyword(operation_text, &["تسلم", "receive"]);
                let (destination, channel) = split_word_top_level(
                    tail,
                    if lower.starts_with("receive ") {
                        "from"
                    } else {
                        "من"
                    },
                )
                .ok_or_else(|| {
                    vec![diagnostic(
                        header.number,
                        1,
                        "expected select receive source",
                    )]
                })?;
                let destination = destination.trim();
                let target = if starts_any(&destination.to_lowercase(), &["إشارة", "signal"]) {
                    None
                } else {
                    Some(parse_target(header.number, destination)?)
                };
                SelectOperation::Receive {
                    channel: parse_expression(header.number, strip_branch_marker(channel.trim()))?,
                    target,
                }
            } else {
                return Err(vec![diagnostic(
                    header.number,
                    1,
                    "select branch must send or receive",
                )]);
            };
            self.at += 1;
            let mut body = Vec::new();
            while let Some(line) = self.current() {
                if matches_word(&line.text, &["تم", "done"])
                    || starts_any(&line.text.to_lowercase(), &["أو ", "or "])
                {
                    break;
                }
                body.push(self.parse_statement()?);
            }
            branches.push(SelectBranch {
                operation,
                body,
                span: Self::line_span(header.number),
            });
        }
        if branches.is_empty() {
            return Err(vec![diagnostic(
                source.number,
                1,
                "select requires at least one branch",
            )]);
        }
        Ok(Stmt {
            span: Self::line_span(source.number),
            kind: StmtKind::Select(branches),
        })
    }
}

fn insert_unique<T>(
    map: &mut HashMap<String, T>,
    name: String,
    value: T,
    line: usize,
) -> Result<(), Vec<Diagnostic>> {
    if map.insert(name.clone(), value).is_some() {
        Err(vec![diagnostic(
            line,
            1,
            format!("duplicate definition `{name}`"),
        )])
    } else {
        Ok(())
    }
}

fn starts_any(source: &str, prefixes: &[&str]) -> bool {
    prefixes
        .iter()
        .any(|prefix| source.starts_with(&prefix.to_lowercase()))
}

fn matches_word(source: &str, words: &[&str]) -> bool {
    words
        .iter()
        .any(|word| source.trim().eq_ignore_ascii_case(word))
}

fn strip_keyword<'a>(source: &'a str, keywords: &[&str]) -> &'a str {
    let trimmed = source.trim_start();
    for keyword in keywords {
        if trimmed
            .get(..keyword.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(keyword))
        {
            return trimmed[keyword.len()..].trim_start();
        }
    }
    trimmed
}

fn split_first_word(source: &str) -> (&str, &str) {
    source
        .find(char::is_whitespace)
        .map(|at| (&source[..at], source[at..].trim_start()))
        .unwrap_or((source, ""))
}

fn split_once_top_level(source: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut quoted = None;
    let mut escaped = false;
    for (at, ch) in source.char_indices() {
        if let Some(quote) = quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                quoted = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quoted = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if (ch == delimiter || (delimiter == ',' && ch == '،')) && depth == 0 => {
                return Some((&source[..at], &source[at + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}

fn split_top_level(source: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    let mut quoted = None;
    let mut escaped = false;
    for (at, ch) in source.char_indices() {
        if let Some(quote) = quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                quoted = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quoted = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if ch == delimiter && depth == 0 => {
                parts.push(&source[start..at]);
                start = at + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&source[start..]);
    parts
}

fn split_word_top_level<'a>(source: &'a str, word: &str) -> Option<(&'a str, &'a str)> {
    let lower = source.to_lowercase();
    let needle = word.to_lowercase();
    let mut depth = 0usize;
    let mut quoted = None;
    let mut escaped = false;
    for (at, ch) in source.char_indices() {
        if let Some(quote) = quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                quoted = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quoted = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0
            && lower
                .get(at..at + needle.len())
                .is_some_and(|part| part == needle)
        {
            let before = at == 0
                || source[..at]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace);
            let end = at + needle.len();
            let after = end == source.len()
                || source[end..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace);
            if before && after {
                return Some((&source[..at], &source[end..]));
            }
        }
    }
    None
}

fn strip_ellipsis(source: &str) -> (&str, bool) {
    source
        .strip_suffix("...")
        .map(|text| (text.trim_end(), false))
        .unwrap_or((source, true))
}

fn strip_branch_marker(source: &str) -> &str {
    let source = source.trim_end_matches(':').trim();
    if let Some((channel, marker)) = source.rsplit_once(':') {
        if marker.trim().parse::<usize>().is_ok() {
            return channel.trim_end();
        }
    }
    source
}

fn starts_with_quote(source: &str) -> bool {
    source.starts_with('"') || source.starts_with('\'')
}

fn parse_string_literal(line: usize, source: &str) -> Result<String, Vec<Diagnostic>> {
    let trimmed = source.trim();
    let mut chars = trimmed.chars();
    let quote = chars
        .next()
        .filter(|ch| *ch == '"' || *ch == '\'')
        .ok_or_else(|| vec![diagnostic(line, 1, "expected quoted string")])?;
    if !trimmed.ends_with(quote) || trimmed.len() < quote.len_utf8() * 2 {
        return Err(vec![diagnostic(line, 1, "unterminated string")]);
    }
    let inner = &trimmed[quote.len_utf8()..trimmed.len() - quote.len_utf8()];
    let mut result = String::new();
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            result.push(match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            result.push(ch);
        }
    }
    if escaped {
        result.push('\\');
    }
    Ok(result)
}

fn parse_expression_list(line: usize, source: &str) -> Result<Vec<Expr>, Vec<Diagnostic>> {
    if source.trim().is_empty() {
        return Ok(Vec::new());
    }
    split_top_level(source, ',')
        .into_iter()
        .map(|part| parse_expression(line, part.trim()))
        .collect()
}

fn parse_header_call(line: usize, source: &str) -> Result<(String, Vec<String>), Vec<Diagnostic>> {
    let source = source.trim().trim_end_matches(':').trim();
    let open = source
        .find('(')
        .ok_or_else(|| vec![diagnostic(line, 1, "expected parameter list")])?;
    let close = matching_close(source, open)
        .ok_or_else(|| vec![diagnostic(line, open + 1, "expected `)`")])?;
    if !source[close + 1..].trim().is_empty() {
        return Err(vec![diagnostic(
            line,
            close + 2,
            "unexpected text after parameter list",
        )]);
    }
    let name = source[..open].trim().to_owned();
    if name.is_empty() {
        return Err(vec![diagnostic(line, 1, "expected name")]);
    }
    let mut parameters = Vec::new();
    for part in split_top_level(&source[open + 1..close], ',') {
        let parameter = part.trim();
        if parameter.is_empty() {
            continue;
        }
        if parameters.iter().any(|existing| existing == parameter) {
            return Err(vec![diagnostic(line, open + 2, "duplicate parameter")]);
        }
        parameters.push(parameter.to_owned());
    }
    Ok((name, parameters))
}

fn parse_call_expression(
    line: usize,
    source: &str,
) -> Result<(String, Vec<Expr>), Vec<Diagnostic>> {
    let source = source.trim().trim_end_matches(':').trim();
    let open = source
        .find('(')
        .ok_or_else(|| vec![diagnostic(line, 1, "expected call argument list")])?;
    let close = matching_close(source, open)
        .ok_or_else(|| vec![diagnostic(line, open + 1, "expected `)`")])?;
    if !source[close + 1..].trim().is_empty() {
        return Err(vec![diagnostic(
            line,
            close + 2,
            "unexpected text after call",
        )]);
    }
    let name = source[..open].trim().to_owned();
    if name.is_empty() {
        return Err(vec![diagnostic(line, 1, "expected call name")]);
    }
    let args = parse_expression_list(line, &source[open + 1..close])?;
    Ok((name, args))
}

fn parse_target(line: usize, source: &str) -> Result<Target, Vec<Diagnostic>> {
    let source = source.trim();
    let span = Span { line, column: 1 };
    if source.contains('[') {
        let expression = parse_expression(line, source)?;
        if let ExprKind::Index { object, indices } = expression.kind {
            return Ok(Target {
                span,
                kind: TargetKind::Index {
                    object: *object,
                    indices,
                },
            });
        }
        return Err(vec![diagnostic(
            line,
            1,
            "expected an indexed assignment target",
        )]);
    }
    if let Some((field, object)) = source.split_once(" $ ") {
        return Ok(Target {
            span,
            kind: TargetKind::Field {
                object: parse_expression(line, object.trim())?,
                name: field.trim().to_owned(),
            },
        });
    }
    let words: Vec<_> = source.split_whitespace().collect();
    if words.len() == 2 && words[0] != "$" {
        return Ok(Target {
            span,
            kind: TargetKind::Field {
                object: parse_expression(line, words[1])?,
                name: words[0].to_owned(),
            },
        });
    }
    if words.len() == 3 && words[1] == "$" {
        return Ok(Target {
            span,
            kind: TargetKind::Field {
                object: parse_expression(line, words[2])?,
                name: words[0].to_owned(),
            },
        });
    }
    if source.is_empty() || source.contains(char::is_whitespace) {
        return Err(vec![diagnostic(line, 1, "invalid assignment target")]);
    }
    Ok(Target {
        span,
        kind: TargetKind::Variable(source.to_owned()),
    })
}

fn matching_delimiter(source: &str, open: usize, opener: char, closer: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (offset, ch) in source[open..].char_indices() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            value if value == opener => depth += 1,
            value if value == closer => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_assignment(source: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (at, ch) in source.char_indices() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            '=' if depth == 0 => {
                let before = source[..at].chars().next_back();
                let after = source[at + 1..].chars().next();
                if !matches!(before, Some('<' | '>' | '='))
                    && after != Some('=')
                    && after != Some('>')
                {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Number(f64),
    String(String),
    Identifier(String),
    Operator(String),
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Colon,
}

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    column: usize,
}

fn parse_expression(line: usize, source: &str) -> Result<Expr, Vec<Diagnostic>> {
    let tokens = tokenize(line, source)?;
    let mut parser = ExprParser {
        tokens,
        at: 0,
        line,
    };
    let expression = parser.parse_or()?;
    if let Some(token) = parser.peek() {
        return Err(vec![diagnostic(
            line,
            token.column,
            "unexpected token after expression",
        )]);
    }
    Ok(expression)
}

fn tokenize(line: usize, source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut tokens = Vec::new();
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (byte, ch) = chars[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        let column = source[..byte].chars().count() + 1;
        let kind = match ch {
            '(' => {
                i += 1;
                TokenKind::LeftParen
            }
            ')' => {
                i += 1;
                TokenKind::RightParen
            }
            '[' => {
                i += 1;
                TokenKind::LeftBracket
            }
            ']' => {
                i += 1;
                TokenKind::RightBracket
            }
            '{' => {
                i += 1;
                TokenKind::LeftBrace
            }
            '}' => {
                i += 1;
                TokenKind::RightBrace
            }
            ',' | '،' => {
                i += 1;
                TokenKind::Comma
            }
            ':' => {
                i += 1;
                TokenKind::Colon
            }
            '"' | '\'' => {
                let quote = ch;
                i += 1;
                let mut value = String::new();
                let mut closed = false;
                while i < chars.len() {
                    let current = chars[i].1;
                    i += 1;
                    if current == quote {
                        closed = true;
                        break;
                    }
                    if current == '\\' && i < chars.len() {
                        let escaped = chars[i].1;
                        i += 1;
                        value.push(match escaped {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            other => other,
                        });
                    } else {
                        value.push(current);
                    }
                }
                if !closed {
                    return Err(vec![diagnostic(line, column, "unterminated string")]);
                }
                TokenKind::String(value)
            }
            '+' | '-' | '−' | '*' | '/' | '×' | '÷' | '=' | '<' | '>' | '!' => {
                let mut op = if ch == '−' {
                    "-".to_owned()
                } else {
                    ch.to_string()
                };
                i += 1;
                if i < chars.len()
                    && ((ch == '<' && matches!(chars[i].1, '>' | '='))
                        || (ch == '>' && chars[i].1 == '=')
                        || (ch == '!' && chars[i].1 == '=')
                        || (ch == '=' && chars[i].1 == '>'))
                {
                    op.push(chars[i].1);
                    i += 1;
                }
                TokenKind::Operator(op)
            }
            c if c.is_ascii_digit() => {
                let start = i;
                i += 1;
                while i < chars.len()
                    && (chars[i].1.is_ascii_digit()
                        || (chars[i].1 == '.'
                            && i + 1 < chars.len()
                            && chars[i + 1].1.is_ascii_digit()))
                {
                    i += 1;
                }
                let raw: String = chars[start..i].iter().map(|(_, c)| c).collect();
                let value = raw
                    .parse::<f64>()
                    .map_err(|_| vec![diagnostic(line, column, "invalid number")])?;
                TokenKind::Number(value)
            }
            c if c.is_alphanumeric() || c == '_' || c == '$' => {
                let start = i;
                i += 1;
                while i < chars.len() {
                    let next = chars[i].1;
                    if next.is_alphanumeric() || next == '_' || next == '.' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let word: String = chars[start..i].iter().map(|(_, c)| c).collect();
                TokenKind::Identifier(word)
            }
            _ => {
                return Err(vec![diagnostic(
                    line,
                    column,
                    format!("unsupported character `{ch}`"),
                )]);
            }
        };
        tokens.push(Token { kind, column });
    }
    Ok(tokens)
}

struct ExprParser {
    tokens: Vec<Token>,
    at: usize,
    line: usize,
}

impl ExprParser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }
    fn take(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        self.at += usize::from(token.is_some());
        token
    }
    fn operator(&self) -> Option<String> {
        match &self.peek()?.kind {
            TokenKind::Operator(op) => Some(op.to_lowercase()),
            TokenKind::Identifier(word) => Some(word.to_lowercase()),
            _ => None,
        }
    }
    fn consume_one_of(&mut self, accepted: &[&str]) -> bool {
        if self
            .operator()
            .is_some_and(|op| accepted.contains(&op.as_str()))
        {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn parse_or(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.parse_and()?;
        while self.consume_one_of(&["or", "أو", "ولا"]) {
            let op = match self.tokens[self.at - 1].kind {
                TokenKind::Identifier(ref word) if word == "ولا" => BinaryOp::Nor,
                _ => BinaryOp::Or,
            };
            let right = self.parse_and()?;
            left = binary(left, op, right);
        }
        Ok(left)
    }
    fn parse_and(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.parse_equality()?;
        while self.consume_one_of(&["and", "و", "وأيضا", "وأيضاً", "also"]) {
            if matches!(self.peek().map(|token| &token.kind), Some(TokenKind::Identifier(word)) if matches!(word.to_lowercase().as_str(), "also" | "أيضا" | "أيضاً"))
            {
                self.at += 1;
            }
            let right = self.parse_equality()?;
            left = binary(left, BinaryOp::And, right);
        }
        Ok(left)
    }
    fn parse_equality(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = if self.consume_one_of(&["=", "=="]) {
                Some(BinaryOp::Equal)
            } else if self.consume_one_of(&["<>", "!="]) {
                Some(BinaryOp::NotEqual)
            } else if self.consume_one_of(&["is", "هو"]) {
                Some(BinaryOp::IsA)
            } else {
                None
            };
            let Some(op) = op else { break };
            if op == BinaryOp::IsA
                && matches!(self.peek().map(|token| &token.kind), Some(TokenKind::Identifier(word)) if matches!(word.to_lowercase().as_str(), "a" | "نوع"))
            {
                self.at += 1;
            }
            let right = self.parse_comparison()?;
            left = binary(left, op, right);
        }
        Ok(left)
    }
    fn parse_comparison(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.parse_additive()?;
        loop {
            let op = if self.consume_one_of(&["<"]) {
                Some(BinaryOp::Less)
            } else if self.consume_one_of(&["<=", "=<"]) {
                Some(BinaryOp::LessEqual)
            } else if self.consume_one_of(&[">"]) {
                Some(BinaryOp::Greater)
            } else if self.consume_one_of(&[">=", "=>"]) {
                Some(BinaryOp::GreaterEqual)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_additive()?;
            left = binary(left, op, right);
        }
        Ok(left)
    }
    fn parse_additive(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = if self.consume_one_of(&["+"]) {
                Some(BinaryOp::Add)
            } else if self.consume_one_of(&["-"]) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_multiplicative()?;
            left = binary(left, op, right);
        }
        Ok(left)
    }
    fn parse_multiplicative(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.consume_one_of(&["*", "×"]) {
                Some(BinaryOp::Multiply)
            } else if self.consume_one_of(&["/", "÷"]) {
                Some(BinaryOp::Divide)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_unary()?;
            left = binary(left, op, right);
        }
        Ok(left)
    }
    fn parse_unary(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        if self.consume_one_of(&["not", "ليس"]) {
            let value = self.parse_unary()?;
            return Ok(Expr {
                span: value.span,
                kind: ExprKind::Unary {
                    operator: UnaryOp::Not,
                    value: Box::new(value),
                },
            });
        }
        if self.consume_one_of(&["-"]) {
            let value = self.parse_unary()?;
            return Ok(Expr {
                span: value.span,
                kind: ExprKind::Unary {
                    operator: UnaryOp::Negate,
                    value: Box::new(value),
                },
            });
        }
        self.parse_primary()
    }
    fn parse_primary(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let token = self
            .take()
            .ok_or_else(|| vec![diagnostic(self.line, 1, "expected expression")])?;
        let span = Span {
            line: self.line,
            column: token.column,
        };
        match token.kind {
            TokenKind::Number(number) => Ok(Expr {
                span,
                kind: ExprKind::Number(number),
            }),
            TokenKind::String(value) => Ok(Expr {
                span,
                kind: ExprKind::String(value),
            }),
            TokenKind::LeftParen => {
                let expr = self.parse_or()?;
                self.expect(TokenKind::RightParen, "expected `)`")?;
                self.parse_postfix(expr)
            }
            TokenKind::LeftBracket => {
                let mut items = Vec::new();
                if !self.next_is(&TokenKind::RightBracket) {
                    loop {
                        items.push(self.parse_or()?);
                        if !self.consume_kind(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RightBracket, "expected `]`")?;
                self.parse_postfix(Expr {
                    span,
                    kind: ExprKind::Array(items),
                })
            }
            TokenKind::LeftBrace => {
                let mut entries = Vec::new();
                if !self.next_is(&TokenKind::RightBrace) {
                    loop {
                        let key = self.parse_or()?;
                        self.expect_operator(
                            "=>",
                            "expected `=>` between dictionary key and value",
                        )?;
                        let value = self.parse_or()?;
                        entries.push((key, value));
                        if !self.consume_kind(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RightBrace, "expected `}`")?;
                self.parse_postfix(Expr {
                    span,
                    kind: ExprKind::Dictionary(entries),
                })
            }
            TokenKind::Identifier(name) => {
                let lower = name.to_lowercase();
                if lower == "true" || lower == "صحيح" {
                    return Ok(Expr {
                        span,
                        kind: ExprKind::Boolean(true),
                    });
                }
                if lower == "false" || lower == "خطأ" {
                    return Ok(Expr {
                        span,
                        kind: ExprKind::Boolean(false),
                    });
                }
                if self.consume_kind(&TokenKind::LeftParen) {
                    let args = self.parse_arguments_after_open()?;
                    return Ok(Expr {
                        span,
                        kind: ExprKind::Call {
                            name,
                            arguments: args,
                        },
                    });
                }
                if self.consume_kind(&TokenKind::Colon) {
                    let method = match self.take() {
                        Some(Token {
                            kind: TokenKind::Identifier(name),
                            ..
                        }) => name,
                        _ => {
                            return Err(vec![diagnostic(
                                self.line,
                                self.current_column(),
                                "expected method name",
                            )]);
                        }
                    };
                    self.expect(TokenKind::LeftParen, "expected `(` after method name")?;
                    let args = self.parse_arguments_after_open()?;
                    return Ok(Expr {
                        span,
                        kind: ExprKind::MethodCall {
                            object: Box::new(Expr {
                                span,
                                kind: ExprKind::Variable(name),
                            }),
                            name: method,
                            arguments: args,
                        },
                    });
                }
                if let Some((consumed, extra_words)) = self.new_constructor_words() {
                    self.at += consumed;
                    let type_name = if extra_words.is_empty() {
                        name
                    } else {
                        format!("{name} {extra_words}")
                    };
                    let normalized = type_name.to_lowercase();
                    let is_widget = [
                        "button",
                        "زر",
                        "window",
                        "نافذة",
                        "box",
                        "صندوق",
                        "text box",
                        "text line",
                        "line edit",
                        "label",
                        "ملصق",
                        "list box",
                        "combo box",
                        "check box",
                        "option group",
                        "group",
                        "مجموعة",
                        "اختيار",
                    ]
                    .iter()
                    .any(|word| normalized.contains(word));
                    return Ok(Expr {
                        span,
                        kind: if is_widget {
                            ExprKind::NewWidget(type_name)
                        } else {
                            ExprKind::NewObject(type_name)
                        },
                    });
                }
                if matches!(self.tokens.get(self.at).map(|token| &token.kind), Some(TokenKind::Identifier(word)) if word == "$")
                {
                    self.at += 1;
                    let object = if self.next_is(&TokenKind::LeftParen) {
                        self.at += 1;
                        let object = self.parse_or()?;
                        self.expect(TokenKind::RightParen, "expected `)` after field receiver")?;
                        object
                    } else {
                        self.parse_primary()?
                    };
                    return Ok(Expr {
                        span,
                        kind: ExprKind::Field {
                            object: Box::new(object),
                            name,
                        },
                    });
                }
                if let Some(Token {
                    kind: TokenKind::Identifier(object_name),
                    ..
                }) = self.tokens.get(self.at)
                {
                    let lower_object = object_name.to_lowercase();
                    if !matches!(
                        lower_object.as_str(),
                        "and"
                            | "or"
                            | "also"
                            | "is"
                            | "new"
                            | "جديد"
                            | "و"
                            | "أو"
                            | "وأيضا"
                            | "وأيضاً"
                    ) {
                        let object_name = object_name.clone();
                        self.at += 1;
                        return Ok(Expr {
                            span,
                            kind: ExprKind::Field {
                                object: Box::new(Expr {
                                    span,
                                    kind: ExprKind::Variable(object_name),
                                }),
                                name,
                            },
                        });
                    }
                }
                self.parse_postfix(Expr {
                    span,
                    kind: ExprKind::Variable(name),
                })
            }
            _ => Err(vec![diagnostic(self.line, token.column, "expected value")]),
        }
    }
    fn parse_arguments_after_open(&mut self) -> Result<Vec<Expr>, Vec<Diagnostic>> {
        let mut args = Vec::new();
        if !self.next_is(&TokenKind::RightParen) {
            loop {
                args.push(self.parse_or()?);
                if !self.consume_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightParen, "expected `)`")?;
        Ok(args)
    }
    fn parse_postfix(&mut self, mut expression: Expr) -> Result<Expr, Vec<Diagnostic>> {
        loop {
            if self.consume_kind(&TokenKind::LeftBracket) {
                let indices = self.parse_expression_items(TokenKind::RightBracket)?;
                expression = Expr {
                    span: expression.span,
                    kind: ExprKind::Index {
                        object: Box::new(expression),
                        indices,
                    },
                };
            } else if self.consume_kind(&TokenKind::Colon) {
                let method = match self.take() {
                    Some(Token {
                        kind: TokenKind::Identifier(name),
                        ..
                    }) => name,
                    _ => {
                        return Err(vec![diagnostic(
                            self.line,
                            self.current_column(),
                            "expected method name",
                        )]);
                    }
                };
                self.expect(TokenKind::LeftParen, "expected `(` after method name")?;
                let arguments = self.parse_arguments_after_open()?;
                expression = Expr {
                    span: expression.span,
                    kind: ExprKind::MethodCall {
                        object: Box::new(expression),
                        name: method,
                        arguments,
                    },
                };
            } else {
                break;
            }
        }
        Ok(expression)
    }
    fn parse_expression_items(
        &mut self,
        terminator: TokenKind,
    ) -> Result<Vec<Expr>, Vec<Diagnostic>> {
        let mut items = Vec::new();
        if !self.next_is(&terminator) {
            loop {
                items.push(self.parse_or()?);
                if !self.consume_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(terminator, "expected closing delimiter")?;
        if items.is_empty() {
            return Err(vec![diagnostic(
                self.line,
                self.current_column(),
                "index expects at least one value",
            )]);
        }
        Ok(items)
    }
    fn new_constructor_words(&self) -> Option<(usize, String)> {
        let mut extra_words = Vec::new();
        let mut at = self.at;
        while let Some(Token {
            kind: TokenKind::Identifier(word),
            ..
        }) = self.tokens.get(at)
        {
            if word.eq_ignore_ascii_case("new") || word == "جديد" {
                return Some((at + 1 - self.at, extra_words.join(" ")));
            }
            if matches!(
                word.to_lowercase().as_str(),
                "and" | "or" | "also" | "is" | "و" | "أو" | "وأيضا" | "وأيضاً"
            ) {
                return None;
            }
            extra_words.push(word.clone());
            at += 1;
        }
        None
    }
    fn next_is(&self, kind: &TokenKind) -> bool {
        self.tokens.get(self.at).is_some_and(|token| {
            std::mem::discriminant(&token.kind) == std::mem::discriminant(kind)
        })
    }
    fn consume_kind(&mut self, kind: &TokenKind) -> bool {
        if self.next_is(kind) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<(), Vec<Diagnostic>> {
        if self.consume_kind(&kind) {
            Ok(())
        } else {
            Err(vec![diagnostic(self.line, self.current_column(), message)])
        }
    }
    fn expect_operator(&mut self, op: &str, message: &str) -> Result<(), Vec<Diagnostic>> {
        if self.consume_one_of(&[op]) {
            Ok(())
        } else {
            Err(vec![diagnostic(self.line, self.current_column(), message)])
        }
    }
    fn current_column(&self) -> usize {
        self.peek().map_or_else(
            || self.tokens.last().map_or(1, |token| token.column + 1),
            |token| token.column,
        )
    }
}

fn binary(left: Expr, operator: BinaryOp, right: Expr) -> Expr {
    let span = left.span;
    Expr {
        span,
        kind: ExprKind::Binary {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        },
    }
}

fn matching_close(source: &str, open: usize) -> Option<usize> {
    matching_delimiter(source, open, '(', ')')
}

fn diagnostic(line: usize, column: usize, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        span: Span { line, column },
        message: message.into(),
    }
}
