use num::integer::div_floor;

use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    fs::File,
    io::Write,
};

use color_eyre::{Result, eyre::eyre};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use itertools::Itertools;
use rand::prelude::SliceRandom;
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Flex, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text, ToLine, ToSpan},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use ron::ser::PrettyConfig;
use rusty_words_common::{
    judgement::{TryMethod, check_word},
    model::{WordsDirection, WordsIndex, WordsList, WordsMeta},
    paths::{index_file, root_dir, words_file_exists},
};
use tui_input::Input;
use tui_input::backend::crossterm as input_backend;

pub fn try_list(
    index: &mut WordsIndex,
    id: usize,
    method: TryMethod,
    direction: WordsDirection,
    shuffle: bool,
    reset: bool,
) -> Result<()> {
    let meta = index
        .lists
        .get_mut(id.checked_sub(1).ok_or_else(|| {
            eyre!("Integer underflow when trying list {id}, lists are 1-indexed.")
        })?)
        .ok_or_else(|| eyre!("Could not find list by ID {id}"))?;
    if reset {
        meta.progress = None;
    }
    let words_file = words_file_exists(&root_dir()?, &meta.uuid)?;
    let mut file = File::open(&words_file)?;
    let mut words: WordsList = ron::de::from_reader(&mut file)?;

    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
    }
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = try_tui(&mut words, &mut terminal, meta, &method, direction, shuffle);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    let ser = ron::ser::to_string_pretty(&words, PrettyConfig::default())?;
    write!(&mut File::create(words_file)?, "{ser}")?;

    let ser = ron::ser::to_string_pretty(&index, PrettyConfig::default())?;
    let mut index_file = File::create(index_file()?)?;
    write!(&mut index_file, "{ser}")?;

    res
}

pub fn try_tui(
    list: &mut WordsList,
    terminal: &mut Terminal<impl Write + Backend>,
    meta: &mut WordsMeta,
    method: &TryMethod,
    direction: WordsDirection,
    shuffle: bool,
) -> Result<()> {
    if list.0.is_empty() {
        return Ok(());
    }
    let total_words = list.0.len();
    let mut n = meta.progress.unwrap_or(0);
    let mut shuffle_map = HashMap::new();

    if shuffle {
        let mut random_array = (0..total_words).collect_vec();
        let mut rng = rand::thread_rng();
        random_array.shuffle(&mut rng);
        shuffle_map = meta
            .shuffle_map
            .insert(HashMap::from_iter((0..total_words).zip(random_array)))
            .clone();
    }

    let mut rotation: VecDeque<_> = (0..10)
        .filter_map(|x| {
            let index = *shuffle_map.get(&x).unwrap_or(&x);
            // Sorry for the clone
            list.0.get(index).map(|x| (index, x.clone(), 0))
        })
        .collect();

    // TODO: Make this configurable
    let total_progress: usize = 3;
    let td_progress = div_floor(total_progress, 2);
    let tui_total = total_words.to_string();

    let term_lang = meta.terms.to_string();
    let def_lang = meta.definition.to_string();

    let mut message = Vec::new();
    while n < total_words {
        let (index, front, mut progress) = rotation.pop_front().unwrap();
        let mut ask = front.terms.as_slice();
        let mut ans = front.definitions.as_slice();
        let direction = direction & front.direction;
        match direction {
            WordsDirection::DT => std::mem::swap(&mut ask, &mut ans),
            WordsDirection::Both if progress > td_progress => std::mem::swap(&mut ask, &mut ans),
            _ => (),
        };
        let tui_direc = match direction {
            WordsDirection::Auto => WordsDirection::TD,
            WordsDirection::Both if progress > td_progress => WordsDirection::DT,
            WordsDirection::Both => WordsDirection::TD,
            e => e,
        };
        let app = App {
            message: &message.into(),
            meta,
            n: &n.to_string(),
            total_words: &tui_total,
            direction: &tui_direc.to_string(),
            ask,
            ans,
            term_lang: &term_lang,
            def_lang: &def_lang,
        };
        let (is_correct, guess) = match method {
            TryMethod::Write => write_and_check(terminal, app, list, &shuffle_map),
            TryMethod::Mpc => todo!(),
        }?;
        let ask = ask.join(", ");
        let ans = ans.join(", ");
        if is_correct {
            message = vec![
                Line::styled("Correct! ", Style::default().fg(Color::Green)),
                Line::raw(format!("{} -> {}", ask, ans)),
            ];

            list.0[index].times_answered_correctly += 1;
            progress += 1;
            if progress == total_progress {
                n += 1;
                let rot = rotation.len();
                if n <= total_words.saturating_sub(rot) {
                    // We can add another word
                    let next = n + rot - 1;
                    let index = *shuffle_map.get(&next).unwrap_or(&next);
                    rotation.push_back((index, list.0.get(index).unwrap().clone(), 0));
                }
                continue;
            }
        } else {
            message = vec![
                Line::styled("Wrong! ", Style::default().fg(Color::Red)),
                Line::raw(format!("{} -> {}. You guessed ", ask, ans)),
                Line::styled(
                    guess,
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ];
        }
        rotation.push_back((index, front, progress));
    }
    Ok(())
}

type AppTerms<'a> = &'a [Cow<'a, str>];
struct App<'a> {
    message: &'a Text<'a>,
    meta: &'a mut WordsMeta,
    /// The progress that has been made (stored in a string so you don't have to tostring it
    /// multiple times per word)
    n: &'a str,
    total_words: &'a str,
    direction: &'a str,
    ask: AppTerms<'a>,
    ans: AppTerms<'a>,
    term_lang: &'a str,
    def_lang: &'a str,
}

fn write_and_check<B: Backend>(
    terminal: &mut Terminal<B>,
    app: App<'_>,
    list: &mut WordsList,
    shuffle_map: &HashMap<usize, usize>,
) -> Result<(bool, String)> {
    let mut input: Input = String::new().into();
    loop {
        terminal.draw(|f| write_ui(f, &app, &input))?;
        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Enter, _) => {
                    break;
                }
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    list.0.sort_unstable_by(|x, y| {
                        x.times_answered_correctly.cmp(&y.times_answered_correctly)
                    });
                    app.meta.progress = Some(
                        app.n
                            .parse()
                            .expect("Should always be a valid number given by callee"),
                    );
                    app.meta.shuffle_map = Some(shuffle_map.clone());
                    return Err(eyre!("User quit"));
                }
                _ => {
                    input_backend::to_input_request(&Event::Key(key)).and_then(|x| input.handle(x));
                }
            }
        }
    }
    let res = check_word(&TryMethod::Write, input.value(), app.ans);
    Ok((res, input.into()))
}

fn write_ui<'a>(f: &'a mut Frame, app: &'a App<'a>, input: &'a Input) {
    let bold = || Style::default().add_modifier(Modifier::BOLD);
    let header_msg = Text::from(vec![
        Line::from(vec![
            Span::raw(app.n),
            Span::styled(" / ", bold()),
            Span::raw(app.total_words.to_string()),
        ]),
        Line::from(vec![
            Span::raw("Direction: "),
            Span::styled(app.direction.to_string(), bold()),
        ]),
        Line::from(vec![
            Span::raw("Terms: "),
            Span::styled(app.term_lang, bold()),
        ]),
        Line::from(vec![
            Span::raw("Definitions: "),
            Span::styled(app.def_lang, bold()),
        ]),
        app.message.to_line(),
    ]);
    let chunks = Layout::default()
        .flex(Flex::Center)
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_msg.lines.len() as u16 + 2),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(f.area());
    let header = Paragraph::new(header_msg)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.meta.name.as_str()),
        )
        .alignment(Alignment::Center);

    let lang = match app.direction {
        "term -> definition" => app.term_lang,
        "definition -> term" => app.def_lang,
        _ => unreachable!("Should have been filtered out at `try_tui`."),
    };
    let ask = Paragraph::new(format!("{} ({})", app.ask.join(", "), lang))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    // TODO: Position cursor instead of append |
    let input_view = Paragraph::new(input.to_span())
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .wrap(Wrap { trim: true });
    f.set_cursor_position((
        chunks[2].x + input.visual_cursor() as u16 + 1,
        chunks[2].y + 1,
    ));

    f.render_widget(header, chunks[0]);
    f.render_widget(ask, chunks[1]);
    f.render_widget(input_view, chunks[2]);
}
