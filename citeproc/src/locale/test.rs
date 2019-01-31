use std::collections::HashMap;
use std::sync::Arc;

use csl::locale::*;
use csl::terms::*;

use super::db::LocaleDatabase;
use super::fetcher::Predefined;
use crate::db_impl::RootDatabase;

fn en_au() -> Lang {
    Lang::Iso(IsoLang::English, Some(IsoCountry::AU))
}

fn terms(xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
        <locale xmlns="http://purl.org/net/xbiblio/csl" version="1.0" xml:lang="en-US">
        <terms>{}</terms></locale>"#,
        xml
    )
}

fn predefined_xml(pairs: &[(Lang, &str)]) -> Predefined {
    let mut map = HashMap::new();
    for (lang, ts) in pairs {
        map.insert(lang.clone(), terms(ts));
    }
    Predefined(map)
}

fn term_and(form: TermFormExtended) -> SimpleTermSelector {
    SimpleTermSelector::Misc(MiscTerm::And, form)
}

fn test_simple_term(term: SimpleTermSelector, langs: &[(Lang, &str)], expect: Option<&str>) {
    let db = RootDatabase::safe_default(Arc::new(predefined_xml(langs)));
    // use en-AU so it has to do fallback to en-US
    let locale = db.merged_locale(en_au());
    assert_eq!(
        locale.get_text_term(TextTermSelector::Simple(term), false),
        expect
    )
}

#[test]
fn term_override() {
    test_simple_term(
        term_and(TermFormExtended::Long),
        &[
            (Lang::en_us(), r#"<term name="and">USA</term>"#),
            (en_au(), r#"<term name="and">Australia</term>"#),
        ],
        Some("Australia"),
    )
}

#[test]
fn term_form_refine() {
    test_simple_term(
        term_and(TermFormExtended::Long),
        &[
            (Lang::en_us(), r#"<term name="and">USA</term>"#),
            (en_au(), r#"<term name="and" form="short">Australia</term>"#),
        ],
        Some("USA"),
    );
    test_simple_term(
        term_and(TermFormExtended::Short),
        &[
            (Lang::en_us(), r#"<term name="and">USA</term>"#),
            (en_au(), r#"<term name="and" form="short">Australia</term>"#),
        ],
        Some("Australia"),
    );
}

#[test]
fn term_form_fallback() {
    test_simple_term(
        term_and(TermFormExtended::Short),
        &[
            (Lang::en_us(), r#"<term name="and">USA</term>"#),
            (en_au(), r#"<term name="and">Australia</term>"#),
        ],
        Some("Australia"),
    );
    test_simple_term(
        // short falls back to long and skips the "symbol" in a later locale
        term_and(TermFormExtended::Short),
        &[
            (Lang::en_us(), r#"<term name="and">USA</term>"#),
            (
                en_au(),
                r#"<term name="and" form="symbol">Australia</term>"#,
            ),
        ],
        Some("USA"),
    );
    test_simple_term(
        term_and(TermFormExtended::VerbShort),
        &[
            (Lang::en_us(), r#"<term name="and" form="long">USA</term>"#),
            (
                en_au(),
                r#"<term name="and" form="symbol">Australia</term>"#,
            ),
        ],
        Some("USA"),
    );
}

#[test]
fn term_locale_fallback() {
    test_simple_term(
        term_and(TermFormExtended::Long),
        &[
            (Lang::en_us(), r#"<term name="and">USA</term>"#),
            (en_au(), r#""#),
        ],
        Some("USA"),
    )
}
