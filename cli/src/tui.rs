use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    fs::File,
    io::Write,
};

use rusty_words_common::{
    model::{WordsDirection, WordsIndex, WordsList, WordsMeta},
    paths::{root_dir, words_file_exists},
};
use color_eyre::{eyre::eyre, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use rand::prelude::SliceRandom;
use ron::ser::PrettyConfig;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use tui_input::backend::crossterm as input_backend;
use tui_input::Input;

use crate::args::TryMethod;

pub fn try_list(
    index: &mut WordsIndex,
    id: usize,
    method: TryMethod,
    direction: WordsDirection,
    shuffle: bool,
) -> Result<()> {
    let meta = index.get(id)?;
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

    res
}

pub fn try_tui(
    list: &mut WordsList,
    terminal: &mut Terminal<impl Write + Backend>,
    meta: &WordsMeta,
    method: &TryMethod,
    direction: WordsDirection,
    shuffle: bool,
) -> Result<()> {
    if list.0.is_empty() {
        return Ok(());
    }
    let total_words = list.0.len();
    let mut n = 0;
    let mut shuffle_map = HashMap::new();

    if shuffle {
        let mut random_array = (0..total_words).collect_vec();
        let mut rng = rand::thread_rng();
        random_array.shuffle(&mut rng);
        shuffle_map = HashMap::from_iter((0..total_words).zip(random_array));
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
    let td_progress = total_progress.div_floor(2);
    let tui_total = total_words.to_string();

    let term_lang = meta.terms.to_string();
    let def_lang = meta.definition.to_string();

    let mut message = Spans(Vec::new());
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
            message: &message,
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
            TryMethod::Write => write_and_check(terminal, app),
            TryMethod::Mpc => todo!(),
        }?;
        let ask = ask.join(", ");
        let ans = ans.join(", ");
        if is_correct {
            message.0 = vec![
                Span::styled("Correct! ", Style::default().fg(Color::Green)),
                Span::raw(format!("{} -> {}", ask, ans)),
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
            message.0 = vec![
                Span::styled("Wrong! ", Style::default().fg(Color::Red)),
                Span::raw(format!("{} -> {}. You guessed ", ask, ans)),
                Span::styled(
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
    message: &'a Spans<'a>,
    meta: &'a WordsMeta,
    n: &'a str,
    total_words: &'a str,
    direction: &'a str,
    ask: AppTerms<'a>,
    ans: AppTerms<'a>,
    term_lang: &'a str,
    def_lang: &'a str,
}

fn write_and_check<B: Backend>(terminal: &mut Terminal<B>, app: App<'_>) -> Result<(bool, String)> {
    let mut input: Input = String::new().into();
    loop {
        terminal.draw(|f| write_ui(f, &app, input.value()))?;
        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Enter, _) => {
                    break;
                }
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    // TODO: Save state
                    return Err(eyre!("User quit"));
                }
                _ => {
                    input_backend::to_input_request(Event::Key(key)).and_then(|x| input.handle(x));
                }
            }
        }
    }
    let res = check_word(&TryMethod::Write, input.value(), app.ans);
    Ok((res, input.into()))
}

fn write_ui<'a, B: Backend>(f: &'a mut Frame<B>, app: &'a App<'a>, input: &'a str) {
    let bold = || Style::default().add_modifier(Modifier::BOLD);
    let header_msg = vec![
        Spans(vec![
            Span::raw(app.n),
            Span::styled(" / ", bold()),
            Span::raw(app.total_words.to_string()),
        ]),
        Spans(vec![
            Span::raw("Direction: "),
            Span::styled(app.direction.to_string(), bold()),
        ]),
        Spans(vec![
            Span::raw("Terms: "),
            Span::styled(app.term_lang, bold()),
        ]),
        Spans(vec![
            Span::raw("Definitions: "),
            Span::styled(app.def_lang, bold()),
        ]),
        app.message.clone(),
    ];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_msg.len() as u16 + 2),
            Constraint::Percentage(35),
            Constraint::Length(3),
            Constraint::Percentage(35),
            Constraint::Length(3),
        ])
        .split(f.size());
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
    let input_view = Paragraph::new(input)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.set_cursor(chunks[4].x + input.len() as u16 + 1, chunks[4].y + 1);

    f.render_widget(header, chunks[0]);
    f.render_widget(ask, chunks[2]);
    f.render_widget(input_view, chunks[4]);
}

// TODO: Make this more advanced
fn check_word<'a>(method: &TryMethod, input: &'a str, check: &[Cow<'a, str>]) -> bool {
    check.iter().any(|x| match method {
        TryMethod::Write => {
            let parentheses = regex::Regex::new("\\(.*\\)").unwrap();
            let x = x.replace(&parentheses, "");
            x.trim().eq_ignore_ascii_case(input)
        }
        TryMethod::Mpc => input == x,
    }) || input == check.join(", ")
}
