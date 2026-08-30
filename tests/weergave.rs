//! De keuze tussen een donkere en een lichte weergave.
//!
//! Het schakelen zelf gebeurt in de browser. Wat een test zonder browser wél
//! kan vaststellen, staat hier: biedt de kopbalk de keuze aan, wordt een
//! bewaarde keuze toegepast vóór het eerste renderen, kent de stijl beide
//! richtingen, en blijft alles wat de pagina nodig heeft van de NAS zelf komen.

mod common;

use common::Server;

fn server() -> Server {
    let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
    Server::start_in(root, &[])
}

fn assert_ok(response: &str) {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "verwachtte 200 OK, kreeg:\n{}",
        &response[..response.len().min(200)]
    );
}

#[test]
fn de_kopbalk_biedt_de_keuze_aan_zodra_het_script_er_is() {
    let server = server();
    let page = server.get("/");

    assert_ok(&page);

    // De schakelaar staat in de pagina, maar verborgen: zonder JavaScript zou
    // hier een knop staan die niets doet.
    let blok = page
        .split("class=\"kop__thema\"")
        .nth(1)
        .expect("de kopbalk hoort de keuze tussen donker en licht te bevatten");
    let einde = blok.find('>').unwrap_or(blok.len());
    assert!(
        blok[..einde].contains("hidden"),
        "de schakelaar hoort verborgen te zijn tot `app.js` hem aansluit: {}",
        &blok[..einde]
    );

    assert!(
        page.contains("data-thema=\"dark\"") && page.contains("data-thema=\"light\""),
        "beide richtingen horen als knop in de kopbalk te staan"
    );
}

#[test]
fn een_bewaarde_keuze_wordt_toegepast_voor_het_renderen() {
    // Een keuze die pas na het laden wordt toegepast, laat de pagina eerst in
    // de andere modus opflitsen. Het script hoort daarom vóór de stylesheet te
    // staan — en dus vóór alles wat er iets mee doet.
    let server = server();
    let page = server.get("/");

    assert_ok(&page);

    let script = page
        .find("sleeve-thema")
        .expect("de pagina hoort een bewaarde keuze terug te zetten");
    let stylesheet = page
        .find("/static/app.css")
        .expect("de pagina hoort de stijl te laden");

    assert!(
        script < stylesheet,
        "de themakeuze hoort vóór de stylesheet te staan, anders flitst de pagina op"
    );
}

#[test]
fn de_stijl_kent_beide_richtingen() {
    let server = server();
    let css = server.get("/static/app.css");

    assert_ok(&css);

    // Zonder keuze beslist het systeem, en een uitgesproken keuze wint van het
    // systeem — in beide richtingen.
    assert!(
        css.contains("prefers-color-scheme: light"),
        "zonder keuze hoort de systeemvoorkeur te gelden"
    );
    assert!(
        css.contains(":root:not([data-thema=\"dark\"])"),
        "een keuze voor donker hoort de systeemvoorkeur opzij te zetten"
    );
    assert!(
        css.contains(":root[data-thema=\"light\"]"),
        "een keuze voor licht hoort de systeemvoorkeur opzij te zetten"
    );
}

#[test]
fn de_pagina_haalt_niets_van_buiten() {
    // De NAS heeft geen internetverbinding. Een lettertype of een script van
    // een CDN zou daar niet aankomen, en de pagina laten wachten op iets wat
    // nooit komt.
    let server = server();

    for pad in ["/", "/static/app.css", "/static/app.js"] {
        let antwoord = server.get(pad);
        assert_ok(&antwoord);

        let inhoud = antwoord
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or(&antwoord);

        for verwijzing in ["http://", "https://", "//fonts."] {
            assert!(
                !inhoud.contains(verwijzing),
                "{pad} verwijst naar iets buiten de NAS: {verwijzing}"
            );
        }
    }
}
