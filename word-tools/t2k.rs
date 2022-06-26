use color_eyre::Result;
use serde::Deserialize;
use serde_xml_rs::{EventReader, ParserConfig};
use std::{
    fmt::Display,
    io::{stdin, BufRead},
};

/// Converts STDIN to t2k on STDOUT
pub(crate) fn to_t2k(font_question: Option<String>, font_answer: Option<String>) -> Result<()> {
    print!(
        "<teach2000><version>853</version><description>Normal</description><message_data mm_files_embedded=\"N\" encrypted=\"N\"><font_question>{}</font_question><font_answer>{}</font_answer><items>",
        font_question.unwrap_or_else(|| String::from("Calibri")),
        font_answer.unwrap_or_else(|| String::from("Calibri"))
    );

    for (id, line) in stdin().lock().lines().enumerate() {
        let line = line?;
        let mut split = line.split('\t');

        let q = split.next().unwrap();
        let a = split.next().unwrap_or_default();

        print!(
            "{}",
            Item {
                id,
                question: q.to_string(),
                answer: a.to_string(),
            }
        );
    }

    println!("</items><testresults /><mapquizfile /></message_data></teach2000>");
    Ok(())
}

// Does the opposite of to_t2k
pub(crate) fn to_tsv() -> Result<()> {
    let t2k = EventReader::new_with_config(
        std::io::stdin(),
        ParserConfig::default().ignore_end_of_stream(true),
    );
    let t2k = Teach2000::deserialize(&mut serde_xml_rs::de::Deserializer::new(t2k))?;
    for T2kItem { questions, answers } in t2k.message_data.items.into_iter() {
        if questions.is_empty() || answers.is_empty() {
            continue;
        }
        println!("{}\t{}", questions[0], answers[0]);
    }
    Ok(())
}

#[derive(Deserialize)]
struct Teach2000 {
    message_data: MessageData,
}

#[derive(Deserialize)]
struct MessageData {
    #[serde(rename = "$value")]
    items: Vec<T2kItem>,
}

#[derive(Deserialize, Default)]
struct T2kItem {
    questions: Vec<String>,
    answers: Vec<String>,
}

#[derive(Debug, Clone)]
struct Item {
    id: usize,
    question: String,
    answer: String,
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<item id=\"{}\"><questions><question id=\"0\">{}</question></questions><answers type=\"0\"><answer id=\"0\">{}</answer></answers><errors>0</errors><testcount>0</testcount></item>",
            self.id, self.question, self.answer
        )
    }
}
