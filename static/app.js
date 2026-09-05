// De toevoegingen die de UI in de browser krijgt: laten zien dat een
// schrijfactie bezig is, een hoes kunnen neerslepen, een reeks bestanden in
// twee klikken kunnen selecteren, en kunnen kiezen tussen een donkere en een
// lichte weergave.
//
// Waarom dit nodig is: een tagwijziging in een FLAC van enkele gigabytes duurt
// minuten. `atomic::replace` kopieert eerst het hele bestand, en dat is geen
// verspilling maar de prijs van de garantie dat het origineel nooit halverwege
// kapot is. In die tijd staat de pagina stil — zonder dit bestand ziet een
// gebruiker niet of de app bezig is of vastzit, en start een tweede klik een
// tweede schrijfactie op hetzelfde bestand.
//
// Dit is een toevoeging, geen voorwaarde: zonder JavaScript versturen de
// formulieren zich precies zoals ze dat altijd deden.
//
// Twee soorten formulieren komen hier langs. Het bewerkformulier en de
// hoespagina versturen zich gewoon en laden een nieuwe pagina; daar blijft de
// bezig-staat staan tot het antwoord er is. De albumweergave gaat via htmx en
// vervangt zichzelf door het antwoord — daar verdwijnt de bezig-staat met het
// oude formulier mee, en hoeft er niets opgeruimd te worden.

(function () {
  "use strict";

  // ── Donker of licht ──────────────────────────────────────────────────────
  //
  // De keuze staat in `localStorage` en wordt al door een klein script in de
  // <head> toegepast, vóór het eerste renderen; hier wordt alleen de
  // schakelaar aangesloten en zichtbaar gemaakt. Zolang er geen keuze is
  // gemaakt, beslist de systeemvoorkeur — en dat blijft zo wanneer dit bestand
  // niet geladen wordt.

  var THEMA_SLEUTEL = "sleeve-thema";

  /** De bewaarde keuze, of `null` wanneer er geen (geldige) keuze staat. */
  function bewaardThema() {
    try {
      var thema = localStorage.getItem(THEMA_SLEUTEL);
      return thema === "dark" || thema === "light" ? thema : null;
    } catch (e) {
      return null;
    }
  }

  /** Wat er nú geldt: de keuze, en anders wat het systeem voorschrijft. */
  function huidigThema() {
    var gekozen = bewaardThema();
    if (gekozen) {
      return gekozen;
    }
    var licht =
      typeof window.matchMedia === "function" &&
      window.matchMedia("(prefers-color-scheme: light)").matches;
    return licht ? "light" : "dark";
  }

  /** Laat op de schakelaar zien welke van de twee aan staat. */
  function toonThema(schakelaar, thema) {
    var knoppen = schakelaar.querySelectorAll("[data-thema]");
    for (var i = 0; i < knoppen.length; i++) {
      var aan = knoppen[i].getAttribute("data-thema") === thema;
      knoppen[i].setAttribute("aria-pressed", aan ? "true" : "false");
    }
  }

  (function themaSchakelaar() {
    var schakelaar = document.getElementById("thema");
    if (!schakelaar) {
      return;
    }

    schakelaar.hidden = false;
    toonThema(schakelaar, huidigThema());

    schakelaar.addEventListener("click", function (event) {
      var doel = event.target;
      var knop = doel instanceof Element ? doel.closest("[data-thema]") : null;
      if (!knop) {
        return;
      }

      var thema = knop.getAttribute("data-thema");
      document.documentElement.dataset.thema = thema;
      try {
        localStorage.setItem(THEMA_SLEUTEL, thema);
      } catch (e) {
        // Een browser die opslag weigert, houdt de keuze voor deze pagina.
      }
      toonThema(schakelaar, thema);
    });
  })();

  // De telling en gevolgen komen van de server, maar tekstvelden versturen
  // zichzelf niet. Ververs alleen de uitleg: het formulier vervangen zou de
  // focus, een volgende toetsaanslag of een klik op Voorbeeld kunnen verliezen.
  var berekeningen = new WeakMap();

  function werkGevolgenBij(event) {
    var veld = event.target;
    if (!(veld instanceof HTMLInputElement) || veld.type !== "text") {
      return;
    }
    var form = veld.closest("form#album");
    if (!form || !form.querySelector(".balk__telling")) {
      return;
    }

    var vorige = berekeningen.get(form);
    if (vorige) {
      window.clearTimeout(vorige.timer);
      vorige.controller.abort();
    }
    var berekening = { controller: new AbortController(), timer: null };
    berekeningen.set(form, berekening);
    form.querySelector(".balk__telling").textContent = "Wijzigingen worden berekend…";

    berekening.timer = window.setTimeout(function () {
      // Geen submitknop meesturen: deze aanvraag berekent alleen de gevolgen.
      var invoer = new URLSearchParams(new FormData(form));
      fetch(form.action, {
        method: "POST",
        headers: { "HX-Request": "true" },
        body: invoer,
        signal: berekening.controller.signal
      })
        .then(function (antwoord) {
          if (!antwoord.ok) {
            throw new Error("De berekening is mislukt");
          }
          return antwoord.text();
        })
        .then(function (html) {
          if (!form.isConnected || berekeningen.get(form) !== berekening) {
            return;
          }
          var pagina = new DOMParser().parseFromString(html, "text/html");
          [".gevolg", ".balk__telling"].forEach(function (selector) {
            var nieuw = pagina.querySelector(selector);
            var huidig = form.querySelector(selector);
            if (nieuw && huidig) {
              huidig.innerHTML = nieuw.innerHTML;
            }
          });
        })
        .catch(function (fout) {
          if (fout.name !== "AbortError" && form.isConnected &&
              berekeningen.get(form) === berekening) {
            form.querySelector(".balk__telling").textContent =
              "De telling kon niet worden bijgewerkt. Bekijk de wijzigingen via Voorbeeld en opslaan.";
          }
        });
    }, event.type === "change" ? 0 : 200);
  }

  document.addEventListener("input", werkGevolgenBij);
  document.addEventListener("change", werkGevolgenBij);

  // ── Bezig met schrijven ──────────────────────────────────────────────────

  // Alleen knoppen die werkelijk schrijven dragen dit attribuut; de waarde is
  // de tekst die de knop tijdens het werk toont. Zo staat in de template, naast
  // de knop zelf, of een actie traag genoeg is om te melden — en hoeft dit
  // bestand geen enkele actienaam te kennen.
  var BEZIG_ATTRIBUUT = "data-bezig";

  /** Markering op het formulier zolang er een verzoek loopt. */
  var BEZIG_KLASSE = "is-bezig";

  var TOELICHTING =
    "Een groot bestand wordt in zijn geheel gekopieerd voordat het vervangen " +
    "wordt; bij enkele gigabytes duurt dat een paar minuten. Laat dit venster " +
    "open staan.";

  document.addEventListener("submit", function (event) {
    var form = event.target;
    if (!(form instanceof HTMLFormElement)) {
      return;
    }

    // Al bezig: dit is een tweede klik, of Enter in een tekstveld terwijl de
    // eerste schrijfactie nog loopt. Het verzoek gaat niet uit.
    if (form.classList.contains(BEZIG_KLASSE)) {
      event.preventDefault();
      return;
    }

    // `submitter` is de knop waarop geklikt is. Ontbreekt hij (Enter in een
    // veld), dan geldt de eerste knop die zou schrijven.
    var knop = event.submitter || form.querySelector("[" + BEZIG_ATTRIBUUT + "]");
    if (!knop || !knop.hasAttribute(BEZIG_ATTRIBUUT)) {
      // Een hulpactie: selecteren, hernummeren, annuleren. Die zijn meteen
      // klaar en horen geen spinner te krijgen.
      return;
    }

    form.classList.add(BEZIG_KLASSE);
    toonBezig(form, knop);

    // Een formulier dat de pagina niet mag verlaten, gaat op de achtergrond.
    // Dat is het hoesje op de bewerkpagina: navigeren zou de tagvelden wegvagen
    // die de gebruiker misschien net heeft ingevuld en nog niet heeft
    // opgeslagen.
    if (form.hasAttribute("data-inplace")) {
      event.preventDefault();
      verstuurInPlaats(form, knop);
    }
  });

  // Firefox en Safari halen een pagina uit de bfcache terug zoals hij was —
  // inclusief uitgeschakelde knoppen. Wie op "terug" drukt, hoort geen dood
  // formulier aan te treffen.
  window.addEventListener("pageshow", function (event) {
    if (event.persisted) {
      herstel();
    }
  });

  /** Zet de knop op "bezig" en meldt eronder waarom het even duurt. */
  function toonBezig(form, knop) {
    // Het oorspronkelijke opschrift bewaren, zodat `herstel` de knop kan
    // teruggeven zoals hij was.
    knop.setAttribute("data-was", knop.textContent);
    knop.innerHTML =
      '<span class="knop__spinner" aria-hidden="true"></span>' +
      escape(knop.getAttribute(BEZIG_ATTRIBUUT));

    // Pas ná deze tik uitschakelen. De browser stelt het verzoek samen tussen
    // deze functie en de volgende taak in de wachtrij; een knop die nú al
    // `disabled` is, stuurt zijn `name`/`value` niet mee — en juist daaraan
    // ziet de server welke actie bedoeld was.
    window.setTimeout(function () {
      var knoppen = form.querySelectorAll("button, input[type=submit]");
      for (var i = 0; i < knoppen.length; i++) {
        knoppen[i].disabled = true;
      }
    }, 0);

    form.appendChild(maakMelding());
  }

  /** De uitleg onder het formulier, als statusregel voor schermlezers. */
  function maakMelding() {
    var melding = document.createElement("p");
    melding.className = "bezigmelding";
    melding.setAttribute("role", "status");
    melding.textContent = TOELICHTING;
    return melding;
  }

  /** Draait alles terug wat `toonBezig` heeft gedaan. */
  function herstel() {
    var formulieren = document.querySelectorAll("form." + BEZIG_KLASSE);
    for (var i = 0; i < formulieren.length; i++) {
      herstelFormulier(formulieren[i]);
    }
  }

  /** Zet één formulier terug in de toestand van vóór de schrijfactie. */
  function herstelFormulier(form) {
    form.classList.remove(BEZIG_KLASSE);

    var knoppen = form.querySelectorAll("button, input[type=submit]");
    for (var i = 0; i < knoppen.length; i++) {
      knoppen[i].disabled = false;
    }

    var melding = form.querySelector(".bezigmelding");
    if (melding) {
      melding.remove();
    }

    // De opschriften komen uit de bfcache zoals ze waren; alleen de knop die we
    // hebben omgezet, is niet meer wat hij was. Een herladen is hier te grof —
    // de tekst staat in `data-was`, gezet bij het omzetten.
    var omgezet = form.querySelectorAll("[data-was]");
    for (var j = 0; j < omgezet.length; j++) {
      omgezet[j].textContent = omgezet[j].getAttribute("data-was");
      omgezet[j].removeAttribute("data-was");
    }
  }

  /** Maakt tekst veilig voor gebruik in HTML. */
  function escape(tekst) {
    var doos = document.createElement("span");
    doos.textContent = tekst;
    return doos.innerHTML;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Een hoes neerslepen
  //
  // Het slepen vult het bestandsveld dat er al staat, en verder niets: de
  // vinkjes en de knoppen bepalen nog steeds wat er met de afbeelding gebeurt.
  // Er wordt dus niets geüpload voordat er op een knop is gedrukt.
  //
  // Zonder JavaScript blijft de bestandsinvoer gewoon werken. De uitnodiging om
  // te slepen staat daarom `hidden` in de template en komt hier tevoorschijn:
  // een hint die nergens toe leidt is erger dan geen hint.
  //
  // Eén vak is de uitzondering: het hoespaneel naast de bestandslijst. Het veld
  // daar draagt geen `name` en staat `hidden`, want zonder dit script zou er
  // een hoes gekozen kunnen worden die nergens heen gaat. Wat er wél gekozen
  // wordt, blijft hier staan tot de voorbeeldweergave in beeld komt — de enige
  // stap die schrijft, en dus de enige waarin de afbeelding meereist.
  // ─────────────────────────────────────────────────────────────────────────

  /** Wat er ingebed mag worden; dezelfde twee die `art::prepare` accepteert. */
  var TOEGESTAAN = ["image/jpeg", "image/png"];

  /** Markering op het vak zolang er iets boven zweeft. */
  var SLEEP_KLASSE = "is-sleepdoel";

  /**
   * De hoes die in het paneel naast de bestandslijst is gekozen.
   *
   * Die afbeelding reist niet met de albumweergave mee: dat formulier post
   * zichzelf bij elk vinkje opnieuw, en megabytes horen niet bij iedere klik
   * over de lijn te gaan. Het bestandsveld in het paneel draagt daarom geen
   * `name` en wordt nooit verstuurd; wat er gekozen is, staat hier tot de
   * voorbeeldweergave in beeld komt. Daar gaat het in het veld dat wél
   * verstuurd wordt, zodat de hoes precies één keer over de lijn gaat: in de
   * stap die werkelijk schrijft.
   */
  var gekozenHoes = null;

  /** Of de bewaking tegen een bestand naast het vak al aan staat. */
  var bewaaktSlepen = false;

  document.addEventListener("DOMContentLoaded", function () {
    sluitAan(document);
  });

  // Wat htmx binnenhaalt, is nieuw en nog nergens op aangesloten: zonder dit
  // valt het slepen stil zodra de albumweergave zichzelf heeft vervangen.
  document.addEventListener("htmx:load", function (event) {
    var wortel = (event.detail && event.detail.elt) || event.target;
    if (!wortel || !wortel.querySelectorAll) {
      return;
    }

    // Er is zojuist geschreven: de hoes zit nu in de bestanden en hoort niet
    // stilzwijgend voor een volgende ronde klaar te blijven staan.
    if (wortel.querySelector(".resultaat")) {
      gekozenHoes = null;
    }

    sluitAan(wortel);
  });

  /** Sluit de neerzetvakken in dit stuk pagina aan. */
  function sluitAan(wortel) {
    var vakken = wortel.querySelectorAll("[data-neerzetvak]");

    for (var i = 0; i < vakken.length; i++) {
      var vak = vakken[i];

      // Hetzelfde vak twee keer aansluiten zou elke sleepactie dubbel
      // verwerken; htmx haalt dezelfde inhoud soms opnieuw langs.
      if (vak.getAttribute("data-aangesloten") === "ja") {
        continue;
      }

      if (maakNeerzetvak(vak)) {
        vak.setAttribute("data-aangesloten", "ja");
        herstelGekozen(vak);
      }
    }

    // Een bestand dat naast het vak belandt, zou de browser openen — en dan is
    // de pagina met het half ingevulde formulier weg. Alleen voorkomen wanneer
    // er ook werkelijk een neerzetvak op deze pagina staat.
    if (vakken.length > 0 && !bewaaktSlepen) {
      bewaaktSlepen = true;
      window.addEventListener("dragover", weiger);
      window.addEventListener("drop", weiger);
    }
  }

  /**
   * Zet de onthouden hoes in dit vak.
   *
   * Zo blijft zichtbaar wat er klaarstaat wanneer de albumweergave zichzelf
   * opnieuw opbouwt, en komt de afbeelding in de voorbeeldweergave terecht in
   * het bestandsveld dat daar wél verstuurd wordt.
   */
  function herstelGekozen(vak) {
    if (!gekozenHoes) {
      return;
    }

    var invoer = vak.querySelector("input[type=file]");
    if (!invoer || (invoer.files && invoer.files.length > 0)) {
      return;
    }

    try {
      var overdracht = new DataTransfer();
      overdracht.items.add(gekozenHoes);
      invoer.files = overdracht.files;
    } catch (e) {
      // Een browser die een bestandsveld niet laat vullen: dan staat het veld
      // er nog gewoon om zelf een afbeelding te kiezen.
      return;
    }

    toon(vak, gekozenHoes);
  }

  /** Laat de browser een gesleept bestand niet zelf openen. */
  function weiger(event) {
    event.preventDefault();
  }

  /**
   * Maakt van één vak een sleepdoel; `false` wanneer er niets aan te sluiten
   * viel.
   */
  function maakNeerzetvak(vak) {
    var invoer = vak.querySelector("input[type=file]");
    if (!invoer) {
      return false;
    }

    var hint = vak.querySelector("[data-neerzetvak-hint]");
    if (hint) {
      hint.hidden = false;
    }

    // Een bestandsveld dat `hidden` in de template staat, wacht op deze code:
    // zonder script zou daar een hoes gekozen kunnen worden die nergens heen
    // gaat, want het veld draagt geen `name`.
    invoer.hidden = false;

    vak.addEventListener("dragenter", function (event) {
      event.preventDefault();
      vak.classList.add(SLEEP_KLASSE);
    });

    vak.addEventListener("dragover", function (event) {
      // Zonder dit weigert de browser de drop: de standaardactie van dragover
      // is "hier mag niets neergezet worden".
      event.preventDefault();
      vak.classList.add(SLEEP_KLASSE);
    });

    vak.addEventListener("dragleave", function (event) {
      // `dragleave` vuurt ook bij het oversteken naar een kindelement; alleen
      // opruimen wanneer de muis het vak zelf verlaat.
      if (!vak.contains(event.relatedTarget)) {
        vak.classList.remove(SLEEP_KLASSE);
      }
    });

    vak.addEventListener("drop", function (event) {
      event.preventDefault();
      vak.classList.remove(SLEEP_KLASSE);
      neerzetten(vak, invoer, event.dataTransfer);
    });

    // Ook wie via de bestandskiezer kiest, hoort te zien wát hij gekozen heeft
    // — en te horen dat het te groot is, langs dezelfde weg.
    invoer.addEventListener("change", function () {
      if (!invoer.files || invoer.files.length !== 1) {
        return;
      }

      var bestand = invoer.files[0];
      var bezwaar = bezwaarTegen(vak, bestand);
      if (bezwaar) {
        wijsAf(vak, invoer, bezwaar);
        return;
      }

      onthoud(invoer, bestand);
      toon(vak, bestand);
    });

    return true;
  }

  /**
   * Houdt vast wat er in het hoespaneel gekozen is.
   *
   * Alleen daar: dat veld draagt geen `name` en wordt niet verstuurd, dus
   * zonder dit zou de keuze bij de eerstvolgende klik verdwijnen. Een veld dat
   * wél verstuurd wordt, draagt de afbeelding zelf en heeft dit niet nodig.
   */
  function onthoud(invoer, bestand) {
    if (!invoer.name) {
      gekozenHoes = bestand;
    }
  }

  /**
   * Wijst af wat er is aangeboden, en laat niets halfs achter.
   *
   * Ook het veld wordt geleegd: stond er al een geldige afbeelding klaar, dan
   * zou een knop blijven staan terwijl de melding zegt dat er iets mis is. Eén
   * duidelijke toestand is beter dan twee die elkaar tegenspreken.
   */
  function wijsAf(vak, invoer, reden) {
    invoer.value = "";

    // Ook wat er onthouden was: anders zou een afgewezen afbeelding de vorige
    // stilzwijgend laten staan, terwijl het vak zegt dat er niets klaarstaat.
    if (!invoer.name) {
      gekozenHoes = null;
    }

    verbergGekozen(vak);
    meld(vak, reden);
  }

  /**
   * Wat er tegen dit bestand is, of `null` wanneer het mag.
   *
   * De omvang wordt hier gecontroleerd en niet pas op de server. Een upload
   * boven de grens wordt daar afgekapt terwijl de browser nog aan het versturen
   * is; die gooit het antwoord dan weg en toont een netwerkfout, dus de uitleg
   * die de server meestuurt komt nooit aan. Beter hier tegenhouden, vóór er een
   * byte de deur uit gaat.
   */
  function bezwaarTegen(vak, bestand) {
    if (TOEGESTAAN.indexOf(bestand.type) === -1) {
      return (
        "Alleen JPEG en PNG kunnen als hoes worden ingebed; dit is " +
        (bestand.type || "een onbekend bestandstype") +
        "."
      );
    }

    var grens = parseInt(vak.getAttribute("data-max-mb"), 10);
    if (grens > 0 && bestand.size > grens * 1024 * 1024) {
      return (
        "Deze afbeelding is " +
        leesbareOmvang(bestand.size) +
        "; er gaat hoogstens " +
        grens +
        " MB in. Verklein hem eerst, of verhoog MAX_UPLOAD_MB."
      );
    }

    return null;
  }

  /** Haalt de voorbeeldweergave en alles wat erbij hoorde weer weg. */
  function verbergGekozen(vak) {
    var gekozen = vak.querySelector("[data-neerzetvak-gekozen]");
    if (gekozen) {
      gekozen.hidden = true;
    }

    var klaar = vak.querySelectorAll("[data-neerzetvak-klaar]");
    for (var i = 0; i < klaar.length; i++) {
      klaar[i].hidden = true;
    }
  }

  /** Verwerkt wat er is neergezet. */
  function neerzetten(vak, invoer, overdracht) {
    if (!overdracht || !overdracht.files || overdracht.files.length === 0) {
      // Een sleepactie zonder bestand: tekst uit een ander venster, een link.
      wijsAf(vak, invoer, "Dat is geen bestand. Sleep een JPEG of PNG hierheen.");
      return;
    }

    if (overdracht.files.length > 1) {
      // Er gaat één hoes tegelijk in een bestand. Stilzwijgend de eerste pakken
      // zou betekenen dat de gebruiker een andere afbeelding krijgt dan hij
      // dacht neer te zetten.
      wijsAf(vak, invoer, "Er gaat één hoes tegelijk. Sleep er één, niet meerdere.");
      return;
    }

    var bestand = overdracht.files[0];
    var bezwaar = bezwaarTegen(vak, bestand);
    if (bezwaar) {
      wijsAf(vak, invoer, bezwaar);
      return;
    }

    // Hier komt het bestand in het veld terecht dat het formulier tóch al
    // verstuurt. Vanaf dat moment is er geen verschil meer met een bestand dat
    // via de bestandskiezer is gekozen.
    invoer.files = overdracht.files;
    onthoud(invoer, bestand);
    toon(vak, bestand);
  }

  /** Toont welk bestand er klaarstaat, met een voorbeeldweergave. */
  function toon(vak, bestand) {
    meld(vak, null);

    // Wat er zichtbaar wordt zodra er iets klaarstaat, bepaalt de template.
    // Op de hoespagina staat de knop er al; op de bewerkpagina verschijnt hij
    // pas hier, want daar is het hoesje in rust gewoon een plaatje.
    var klaar = vak.querySelectorAll("[data-neerzetvak-klaar]");
    for (var i = 0; i < klaar.length; i++) {
      klaar[i].hidden = false;
    }

    var gekozen = vak.querySelector("[data-neerzetvak-gekozen]");
    if (!gekozen) {
      return;
    }

    // De voorbeeldweergave bestaat pas zodra er iets te tonen valt: een lege
    // `<img>` in de pagina zou een bestand zonder hoes een afbeelding geven
    // die er nooit komt.
    var voorbeeld = gekozen.querySelector("img");
    var naam = gekozen.querySelector("figcaption");
    if (!voorbeeld) {
      voorbeeld = document.createElement("img");
      voorbeeld.className = "neerzetvak__voorbeeld";
      voorbeeld.width = 96;
      voorbeeld.height = 96;

      naam = document.createElement("figcaption");
      naam.className = "neerzetvak__naam";

      gekozen.appendChild(voorbeeld);
      gekozen.appendChild(naam);
    }

    // De vorige URL vrijgeven; anders houdt de browser elke gekozen afbeelding
    // vast tot de pagina weg is.
    if (voorbeeld.src && voorbeeld.src.indexOf("blob:") === 0) {
      URL.revokeObjectURL(voorbeeld.src);
    }

    voorbeeld.src = URL.createObjectURL(bestand);
    voorbeeld.alt = "Voorbeeld van " + bestand.name;
    naam.textContent = bestand.name + " · " + leesbareOmvang(bestand.size);
    gekozen.hidden = false;
  }

  /** Zet een melding in het vak, of haalt hem weg met `null`. */
  function meld(vak, tekst) {
    var melding = vak.querySelector("[data-neerzetvak-melding]");
    if (!melding) {
      return;
    }

    melding.textContent = tekst || "";
    melding.hidden = !tekst;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Een hoes embedden zonder de pagina te verlaten
  //
  // Het formulier gaat naar dezelfde route als altijd; alleen het antwoord komt
  // hier terecht in plaats van in het venster. Wat de server terugstuurt is de
  // hele hoespagina, inclusief het rapport per bestand — daar wordt de ene
  // regel uit gehaald die deze gebruiker aangaat.
  // ─────────────────────────────────────────────────────────────────────────

  /** Verstuurt het formulier op de achtergrond en werkt de pagina bij. */
  function verstuurInPlaats(form, knop) {
    var velden = new FormData(form);

    // `FormData` neemt de ingedrukte knop niet mee, en juist daaraan ziet de
    // server welke actie bedoeld was.
    if (knop && knop.name) {
      velden.append(knop.name, knop.value);
    }

    fetch(form.action, { method: "POST", body: velden })
      .then(function (antwoord) {
        return antwoord.text().then(function (html) {
          return { ok: antwoord.ok, status: antwoord.status, html: html };
        });
      })
      .then(function (uitkomst) {
        herstelFormulier(form);

        if (!uitkomst.ok) {
          uitkomstTonen(form, samenvatting(uitkomst.html) || "Het is niet gelukt de hoes te plaatsen (" + uitkomst.status + ").", true);
          return;
        }

        var rapport = leesRapport(uitkomst.html);
        uitkomstTonen(form, rapport.tekst, rapport.mislukt);

        if (!rapport.mislukt) {
          verversHoes(form);
          opruimenNaPlaatsen(form);
        }
      })
      .catch(function (fout) {
        herstelFormulier(form);
        uitkomstTonen(form, "Het is niet gelukt de hoes te plaatsen: " + fout.message, true);
      });
  }

  /**
   * Haalt de regel over dit bestand uit het antwoord.
   *
   * De server stuurt de hele hoespagina terug; daarin staat per bestand wat er
   * gebeurd is. Een antwoord met status 200 betekent nog niet dat het gelukt
   * is — een bestand dat niet geschreven kon worden, staat als mislukt in dat
   * rapport.
   */
  function leesRapport(html) {
    var pagina = new DOMParser().parseFromString(html, "text/html");
    var regel = pagina.querySelector(".resultaat__uitkomst");
    var mislukt = pagina.querySelector(".resultaat__bestand--fout") !== null;

    return {
      tekst: regel ? regel.textContent.trim() : "Hoes geplaatst.",
      mislukt: mislukt,
    };
  }

  /** De kale tekst van een foutantwoord, voor zover er iets in staat. */
  function samenvatting(html) {
    var tekst = html.replace(/<[^>]*>/g, " ").replace(/\s+/g, " ").trim();
    return tekst.length > 0 && tekst.length < 300 ? tekst : null;
  }

  /** Zet de uitkomst onder het hoesje. */
  function uitkomstTonen(form, tekst, mislukt) {
    var regel = form.querySelector("[data-uitkomst]");
    if (!regel) {
      return;
    }

    regel.textContent = tekst;
    regel.hidden = false;
    regel.classList.toggle("hoesdoel__uitkomst--fout", !!mislukt);
  }

  /**
   * Laat het hoesje de zojuist geplaatste afbeelding zien.
   *
   * Met een tijdstempel erachter, anders houdt de browser de oude afbeelding
   * vast: het adres is niet veranderd, alleen de inhoud erachter. Had het
   * bestand nog geen hoes, dan staat er een lege plaatshouder die eerst een
   * `<img>` moet worden.
   */
  function verversHoes(form) {
    var adres = form.getAttribute("data-art-url");
    if (!adres) {
      return;
    }

    var vers = adres + (adres.indexOf("?") === -1 ? "?" : "&") + "t=" + Date.now();

    var hoes = form.querySelector("img.bestandskop__hoes");
    if (hoes) {
      hoes.src = vers;
      return;
    }

    var leeg = form.querySelector(".bestandskop__hoes--leeg");
    if (!leeg) {
      return;
    }

    var afbeelding = document.createElement("img");
    afbeelding.className = "bestandskop__hoes";
    afbeelding.src = vers;
    afbeelding.alt = "Album art van dit bestand";
    afbeelding.width = 96;
    afbeelding.height = 96;

    var link = document.createElement("a");
    link.href = leeg.getAttribute("href");
    link.appendChild(afbeelding);

    leeg.replaceWith(link);
  }

  /** Maakt het vak weer leeg, zodat er niet nóg eens dezelfde hoes in gaat. */
  function opruimenNaPlaatsen(form) {
    var invoer = form.querySelector("input[type=file]");
    if (invoer) {
      invoer.value = "";
    }

    verbergGekozen(form);
    meld(form, null);
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Een reeks bestanden selecteren
  //
  // Twintig tracks van een schijf aanvinken is twintig klikken. Klikken op een
  // regel selecteert er één, shift-klikken alles ertussen, en ctrl- of
  // cmd-klikken haalt er één bij of weg — zoals in een bestandsbeheerder.
  //
  // Wat er hier gebeurt, is precies wat een mens met de hand zou doen: dezelfde
  // vinkjes worden gezet, en daarna post het formulier één keer. De selectie
  // blijft dus server-state, en er is geen tweede waarheid over wat er
  // geselecteerd staat. Zonder dit bestand blijven de vinkjes wat ze waren.
  //
  // Een reeks volgt de volgorde waarin de regels in de tabel staan, en niets
  // anders: die volgorde ís de lijst zoals hij er op dat moment uitziet, ook
  // wanneer hij per schijf gegroepeerd is.
  // ──────────────────────────────────────────────────────────────────────────

  /** Een regel in de albumtabel; de kop van een schijf hoort er niet bij. */
  var RIJ = "tr.batchtabel__rij";

  /** Het selectievinkje in zo'n regel. */
  var VAKJE = "input[name=bestand]";

  /** De markering waaraan een gekozen regel te zien is. */
  var GEKOZEN_KLASSE = "batchtabel__rij--gekozen";

  /**
   * Zet het script op de tabel, zodat de opmaak weet dat er te klikken valt.
   *
   * De klasse komt hier vandaan en staat niet in de template: zonder dit
   * bestand hoort een regel er niet uit te zien alsof een klik erop iets doet.
   */
  var SELECTEERBAAR = "batchtabel--selecteerbaar";

  /**
   * De bestandsnaam waar de vorige klik viel: het beginpunt van een reeks.
   *
   * Als naam en niet als element, want de tabel wordt bij elke klik door htmx
   * vervangen — het element van zojuist bestaat dan niet meer. Staat de naam
   * niet meer in de tabel (een andere map), dan telt een shift-klik als een
   * gewone klik.
   */
  var anker = null;

  document.addEventListener("DOMContentLoaded", function () {
    maakSelecteerbaar(document);
  });

  document.addEventListener("htmx:load", function (event) {
    var wortel = (event.detail && event.detail.elt) || event.target;
    if (wortel && wortel.querySelectorAll) {
      maakSelecteerbaar(wortel);
    }
  });

  /** Merkt de albumtabellen in dit stuk pagina als aanklikbaar. */
  function maakSelecteerbaar(wortel) {
    var tabellen = wortel.querySelectorAll(".batchtabel");
    for (var i = 0; i < tabellen.length; i++) {
      tabellen[i].classList.add(SELECTEERBAAR);
    }
  }

  document.addEventListener("click", function (event) {
    var doel = event.target;
    if (!(doel instanceof Element)) {
      return;
    }

    var rij = doel.closest(RIJ);
    if (!rij) {
      return;
    }

    var vakje = rij.querySelector(VAKJE);
    var lijst = regels(rij);
    if (!vakje || lijst.length === 0) {
      return;
    }

    // Het vinkje zelf, of het label eromheen: dat blijft doen wat het deed —
    // één bestand aan of uit, met zijn eigen verzoek. Alleen shift doet er iets
    // bij: de stand die het vinkje zojuist kreeg, gaat over de hele reeks vanaf
    // het anker. Dat is meteen de toetsenbordweg, want shift+spatie op een
    // vinkje geeft een klik met `shiftKey`.
    if (doel.closest("label.vinkje")) {
      if (event.shiftKey) {
        // Het vinkje post zelf, en neemt het hele formulier mee; wat hier
        // gezet wordt reist dus in datzelfde ene verzoek mee.
        toepassen(lijst, reeks(lijst, rij, vakje.checked));
      } else {
        anker = rij.getAttribute("data-bestand");
      }
      return;
    }

    // Een invoerveld, een link, een knop: die hebben hun eigen werk, en een
    // selectie die onder het intikken vandaan verschuift zit dat in de weg.
    if (doel.closest("input, textarea, select, button, a")) {
      return;
    }

    var stand = gewenst(lijst, rij, event);
    if (!event.shiftKey) {
      // Bij shift blijft het anker staan: zo rekt een tweede shift-klik
      // dezelfde reeks op in plaats van er een nieuwe te beginnen.
      anker = rij.getAttribute("data-bestand");
    }

    // De klik viel op de regel en niet op iets wat zelf iets doet; hij hoort
    // de tekst eronder niet te selecteren.
    event.preventDefault();

    if (!toepassen(lijst, stand)) {
      // Deze klik levert de selectie op die er al staat. Dan is er niets te
      // vragen, en gaat er geen verzoek uit.
      return;
    }

    verstuur(rij);
  });

  // Shift-klikken selecteert in een browser de tekst tussen twee punten, en dat
  // is hier niet wat er bedoeld wordt. Alleen tegenhouden waar de klik ook
  // werkelijk een reeks maakt: in een invoerveld blijft shift gewoon selecteren.
  document.addEventListener("mousedown", function (event) {
    if (!event.shiftKey) {
      return;
    }

    var doel = event.target;
    if (!(doel instanceof Element) || !doel.closest(RIJ)) {
      return;
    }

    if (doel.closest("input, textarea, select")) {
      return;
    }

    event.preventDefault();
  });

  /** De regels van de tabel waarin deze regel staat, in de volgorde van de lijst. */
  function regels(rij) {
    var tabel = rij.closest("table");
    return tabel ? Array.prototype.slice.call(tabel.querySelectorAll(RIJ)) : [];
  }

  /** Het vinkje van een regel. */
  function vakjeVan(rij) {
    return rij.querySelector(VAKJE);
  }

  /** Waar in de lijst het bestand met deze naam staat, of `-1`. */
  function plaatsVan(lijst, naam) {
    for (var i = 0; i < lijst.length; i++) {
      if (lijst[i].getAttribute("data-bestand") === naam) {
        return i;
      }
    }
    return -1;
  }

  /**
   * Welke regels er na deze klik geselecteerd horen te staan.
   *
   * Kaal klikken selecteert dit ene bestand en niets anders, ctrl of cmd haalt
   * het erbij of eraf en laat de rest staan, en shift maakt van de reeks tussen
   * het anker en hier de selectie.
   */
  function gewenst(lijst, rij, event) {
    var hier = lijst.indexOf(rij);

    if (event.shiftKey) {
      var vanaf = anker === null ? -1 : plaatsVan(lijst, anker);
      if (vanaf !== -1) {
        return lijst.map(function (_, i) {
          return i >= Math.min(vanaf, hier) && i <= Math.max(vanaf, hier);
        });
      }
      // Zonder anker valt er niets uit te strekken; dan is dit de eerste klik.
    }

    if (event.metaKey || event.ctrlKey) {
      return lijst.map(function (regel, i) {
        var aan = vakjeVan(regel).checked;
        return i === hier ? !aan : aan;
      });
    }

    return lijst.map(function (_, i) {
      return i === hier;
    });
  }

  /**
   * Dezelfde stand over de reeks van het anker tot deze regel, en de rest zoals
   * hij stond.
   *
   * Dit is wat shift bij een vinkje doet: het vinkje bepaalt of de reeks aan of
   * uit gaat, en wat erbuiten valt blijft staan. Zonder anker verandert er
   * niets buiten het vinkje zelf.
   */
  function reeks(lijst, rij, aan) {
    var hier = lijst.indexOf(rij);
    var vanaf = anker === null ? hier : plaatsVan(lijst, anker);
    if (vanaf === -1) {
      vanaf = hier;
    }

    var begin = Math.min(vanaf, hier);
    var eind = Math.max(vanaf, hier);

    return lijst.map(function (regel, i) {
      return i >= begin && i <= eind ? aan : vakjeVan(regel).checked;
    });
  }

  /**
   * Zet de vinkjes op de gevraagde stand; `false` wanneer er niets te doen viel.
   *
   * De markering op de regel gaat mee: het antwoord van de server komt pas over
   * een moment, en tot die tijd hoort te kloppen wat er staat.
   */
  function toepassen(lijst, stand) {
    var veranderd = false;

    for (var i = 0; i < lijst.length; i++) {
      var vakje = vakjeVan(lijst[i]);
      if (!vakje || vakje.checked === stand[i]) {
        continue;
      }

      vakje.checked = stand[i];
      lijst[i].classList.toggle(GEKOZEN_KLASSE, stand[i]);
      veranderd = true;
    }

    return veranderd;
  }

  /**
   * Laat het formulier zich versturen, zoals een vinkje dat ook doet.
   *
   * Via `requestSubmit` en niet via `submit`, want alleen dan komt er een
   * `submit`-event — en daar hangt htmx aan, dat het antwoord in de pagina
   * zet in plaats van ernaartoe te navigeren. Kan een browser dat niet, dan
   * wordt het een gewone POST en komt de hele pagina terug: trager, maar met
   * dezelfde uitkomst.
   */
  function verstuur(rij) {
    var form = rij.closest("form");
    if (!form) {
      return;
    }

    if (typeof form.requestSubmit === "function") {
      form.requestSubmit();
    } else {
      form.submit();
    }
  }

  /** Een bestandsomvang zoals een mens hem leest. */
  function leesbareOmvang(bytes) {
    if (bytes < 1024) {
      return bytes + " B";
    }
    if (bytes < 1024 * 1024) {
      return Math.round(bytes / 1024) + " kB";
    }
    return (bytes / (1024 * 1024)).toFixed(1).replace(".", ",") + " MB";
  }
})();
