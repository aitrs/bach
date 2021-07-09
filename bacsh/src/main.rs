extern crate clap;
use clap::{App, Arg, SubCommand};
extern crate r_i18n;
use r_i18n::{I18n, I18nConfig};
use std::fs::File;
use std::io::prelude::*;
use std::mem::take;
use std::net::TcpStream;

static LOCALE_DIR: &str = "translations";
static LOCALES: [&str; 2] = ["en", "fr"];

macro_rules! setup_locale {
    ($conf: ident, $locale: ident) => {
        $locale = I18n::configure(&$conf);
        let mut f = File::open("/etc/locale.conf")?;
        let mut locont = String::new();
        f.read_to_string(&mut locont)?;
        if locont.contains("fr_") {
            $locale.set_current_lang("fr");
        } else {
            $locale.set_current_lang("en");
        }
    };
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conf = I18nConfig {
        locales: &LOCALES,
        directory: LOCALE_DIR,
    };
    let mut locale: I18n;

    setup_locale!(conf, locale);

    let listsub = SubCommand::with_name("list")
        .about(locale.t("listsubdesc").as_str().unwrap_or(""))
        .arg(
            Arg::with_name("running")
                .short("r")
                .long("running")
                .takes_value(false)
                .help(locale.t("listsubdescrunning").as_str().unwrap_or("")),
        );
    let statussub = SubCommand::with_name("status")
        .about(locale.t("statussubdesc").as_str().unwrap_or(""))
        .arg(
            Arg::with_name("NAME")
                .help(locale.t("statussubdescname").as_str().unwrap_or(""))
                .required(true)
                .index(1),
        );
    let firesub = SubCommand::with_name("fire")
        .about(locale.t("firesubdesc").as_str().unwrap_or(""))
        .arg(
            Arg::with_name("NAME")
                .help(locale.t("firesubdescname").as_str().unwrap_or(""))
                .required(true)
                .index(1),
        );
    let stopsub = SubCommand::with_name("stop")
        .about(locale.t("stopsubdesc").as_str().unwrap_or(""))
        .arg(
            Arg::with_name("NAME")
                .help(locale.t("stopsubdescname").as_str().unwrap_or(""))
                .required(true)
                .index(1),
        );
    let termsub =
        SubCommand::with_name("terminate").about(locale.t("termdesc").as_str().unwrap_or(""));
    let matches = App::new("Bach Shell")
        .version("0.1.0")
        .author("Dorian Vuolo")
        .about(locale.t("desc").as_str().unwrap_or(""))
        .subcommand(listsub.clone())
        .subcommand(statussub.clone())
        .subcommand(firesub.clone())
        .subcommand(stopsub.clone())
        .subcommand(termsub.clone())
        .get_matches();

    let switch_to_shell = matches.subcommand_name().is_none();

    if switch_to_shell {
    } else {
    }

    Ok(())
}
