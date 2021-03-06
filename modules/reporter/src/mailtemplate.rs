use bach_module::ModResult;
use handlebars::Handlebars;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

fn gen_template() -> String {
    "
    <html>
        <head></head>
        <body>
            <h3>Bach Backup Report<h3>
            <p>
                <strong>Overall Status: {{overall}}</strong>
            </p>
            <p>
                <h4>Status Messages</h4>
                {{#each lines}}
                <span>{{this.text}}</span><br />
                {{/each}}
            </p>
        </body>
    </html>
    "
    .to_string()
}

pub fn gen_mail(lines: Vec<String>, template: &Option<PathBuf>) -> ModResult<(String, String)> {
    #[derive(Serialize)]
    struct Line {
        color: String,
        text: String,
    }
    #[derive(Serialize)]
    struct Args {
        overall: String,
        lines: Vec<Line>,
    }
    let mut args: Args = Args {
        overall: "".to_string(),
        lines: Vec::new(),
    };
    let temp_contents = match template {
        Some(path) => fs::read_to_string(path)?,
        None => gen_template(),
    };
    let mut severity_level = 0;

    for l in lines {
        if l.to_lowercase().contains("warning") || l.to_lowercase().contains("warn") {
            if severity_level < 1 {
                severity_level = 1;
            }
            args.lines.push(Line {
                color: "yellow".to_string(),
                text: l.to_string(),
            });
        } else if l.to_lowercase().contains("err") || l.to_lowercase().contains("error") {
            if severity_level < 2 {
                severity_level = 2;
            }
            args.lines.push(Line {
                color: "red".to_string(),
                text: l.to_string(),
            });
        } else {
            args.lines.push(Line {
                color: "default".to_string(),
                text: l.to_string(),
            });
        }
    }

    if severity_level == 0 {
        args.overall = "OK".to_string();
    } else if severity_level == 1 {
        args.overall = "WARNING".to_string();
    } else if severity_level == 2 {
        args.overall = "ERROR".to_string();
    }

    let reg = Handlebars::new();
    let contents = reg.render_template(&temp_contents, &args)?;
    let clean = Regex::new(r"\s{2,}")?
        .replace_all(&contents, "")
        .to_string();
    Ok((
        clean,
        if severity_level == 1 {
            "warning".to_string()
        } else if severity_level == 2 {
            "error".to_string()
        } else {
            "debug".to_string()
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::mailtemplate::gen_mail;
    use bach_module::ModResult;
    #[test]
    fn template_mail_generation() -> ModResult<()> {
        let mail = gen_mail(
            vec![
                "foo".to_string(),
                "Error: bar".to_string(),
                "Warn: baz".to_string(),
            ],
            &None,
        )?;
        assert_eq!(
            mail.0,
            "<html><head></head><body><h3>Bach Backup Report<h3><p><strong>Overall Status: ERROR</strong></p><p><h4>Status Messages</h4><span>foo</span><br /><span>Error: bar</span><br /><span>Warn: baz</span><br /></p></body></html>"
            .to_string()
        );
        Ok(())
    }
}
