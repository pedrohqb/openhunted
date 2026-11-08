// SPDX-License-Identifier: GPL-3.0-or-later
//! As telas `.ITF` da casca sobre os dados reais (OPENHUNTED_DATA): todas montam e
//! desenham; opções, escolher missão, heróis, nome e carro respondem à entrada
//! (`docs/re/menus-restantes.md`, `opcoes-salvamento.md`).

use openhunted_engine::shell::config::Config;
use openhunted_engine::shell::heroes::{
    Car, Heroes, Info, InfoKind, NameEntry, car_ids, info_ids, name_ids,
};
use openhunted_engine::shell::levels::{self, Entry, LevelSelect};
use openhunted_engine::shell::options::{self, Options};
use openhunted_engine::shell::{Dir, Input, Page, Placement, Reply, Screen};
use openhunted_engine::{Assets, Framebuffer};
use openhunted_formats::{Interface, open_data};

fn assets() -> Option<Assets> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    Some(Assets::new(gd.resources().unwrap()))
}

fn screen(a: &mut Assets, name: &str, p: Placement) -> Screen {
    let itf = Interface::parse(a.archive().get(name).unwrap()).unwrap();
    Screen::new(&itf, a, p).unwrap()
}

/// Clica no centro do controle e espera o despacho.
fn click(p: &mut dyn Page, id: u16) -> Option<Reply> {
    let r = p.screen().rect(id).unwrap();
    let mut sounds = Vec::new();
    p.handle(
        Input::PointerMove(r.x + r.w / 2, r.y + r.h / 2),
        &mut sounds,
    );
    let mut got = p.handle(Input::PointerDown, &mut sounds);
    for i in 0..8 {
        if i == 1 {
            got = got.or(p.handle(Input::PointerUp, &mut sounds));
        }
        got = got.or(p.handle(Input::Tick, &mut sounds));
    }
    got
}

fn nonblank(fb: &Framebuffer) -> usize {
    fb.pixels.iter().filter(|&&p| p != 0).count()
}

#[test]
fn every_itf_on_the_disc_builds_and_draws() {
    let Some(mut a) = assets() else { return };
    let names: Vec<String> = a
        .archive()
        .entries()
        .iter()
        .map(|e| e.name.clone())
        .filter(|n| n.ends_with(".ITF"))
        .collect();
    assert_eq!(names.len(), 16);
    for n in names {
        for p in [Placement::Full, Placement::Centered] {
            let mut s = screen(&mut a, &n, p);
            for _ in 0..3 {
                s.handle(Input::Tick);
            }
            let mut fb = Framebuffer::new(640, 480);
            s.draw(&mut fb);
            assert!(nonblank(&fb) > 1000, "{n} desenha");
        }
    }
}

#[test]
fn options_change_and_report_the_config() {
    let Some(mut a) = assets() else { return };
    let s = screen(&mut a, "SOPTI.ITF", Placement::Full);
    let mut o = Options::new(s, Config::default());
    assert_eq!(
        o.screen.frame(options::MEDIUM).unwrap().0,
        2,
        "médio marcado"
    );
    let Some(Reply::Config(c)) = click(&mut o, options::HARD) else {
        panic!()
    };
    assert_eq!(c.difficulty, 2);
    o.handle(Input::Tick, &mut Vec::new());
    assert_eq!(o.screen.frame(options::MEDIUM).unwrap().0, 0);
    assert_eq!(o.screen.frame(options::HARD).unwrap().0, 2);
    let Some(Reply::Config(c)) = click(&mut o, options::MUSIC_UP) else {
        panic!()
    };
    assert_eq!(c.music_volume, 35);
    let Some(Reply::Config(c)) = click(&mut o, options::LEVEL5) else {
        panic!()
    };
    assert_eq!(c.brightness, 1406);
    let Some(Reply::Config(c)) = click(&mut o, options::SFX_BAR) else {
        panic!()
    };
    assert_eq!(c.sfx_volume, 50, "clique no meio da barra");
    let mut fb = Framebuffer::new(640, 480);
    o.draw(&mut fb);
    assert_eq!(click(&mut o, options::DONE), Some(Reply::Close));
    assert_eq!(o.handle(Input::Cancel, &mut Vec::new()), Some(Reply::Close));
}

#[test]
fn level_select_scrolls_marks_and_picks() {
    let Some(mut a) = assets() else { return };
    let font = a.font("SLEVD.PFT").unwrap();
    let s = screen(&mut a, "SLEVL.ITF", Placement::Full);
    let entries: Vec<Entry> = (0..12)
        .map(|i| Entry {
            title: format!("MISSION {i}"),
            won: i % 2 == 0,
            garathe: i == 1,
            colors: None,
        })
        .collect();
    let mut l = LevelSelect::new(s, font, entries, 0);
    let mut snd = Vec::new();
    l.handle(Input::Nav(Dir::Down), &mut snd);
    assert_eq!(l.selected(), 1);
    for _ in 0..8 {
        l.handle(Input::Nav(Dir::Down), &mut snd);
    }
    assert_eq!((l.selected(), l.first()), (9, 3), "rola com a escolha");
    assert_eq!(click(&mut l, levels::LINE1), None);
    assert_eq!(l.selected(), 3, "clique na 1ª linha visível");
    assert_eq!(click(&mut l, levels::UP), None);
    assert_eq!(l.first(), 2);
    assert_eq!(click(&mut l, levels::HEAD2HEAD), Some(Reply::Unavailable));
    let mut fb = Framebuffer::new(640, 480);
    l.draw(&mut fb);
    assert_eq!(click(&mut l, levels::LOAD), Some(Reply::Pick(3)));
    assert_eq!(click(&mut l, levels::CANCEL_BUTTON), Some(Reply::Close));
}

#[test]
fn heroes_name_car_and_info_pages() {
    let Some(mut a) = assets() else { return };
    let font = a.font("SLEVD.PFT").unwrap();
    let s = screen(&mut a, "SHERO.ITF", Placement::Full);
    let mut h = Heroes::new(
        s,
        font.clone(),
        vec![("ANA".into(), 900), ("BIA".into(), 5)],
    );
    let mut snd = Vec::new();
    for _ in 0..5 {
        h.handle(Input::Tick, &mut snd);
    }
    assert_eq!(h.shown(), 2);
    assert_eq!(snd, [3093, 3093], "uma linha por quadro, com som");
    assert_eq!(h.handle(Input::Cancel, &mut snd), Some(Reply::Close));

    let s = screen(&mut a, "SHIGH.ITF", Placement::Centered);
    let mut n = NameEntry::new(s, font.clone(), Some("Save Game As:".into()), "JA");
    n.text("KE é!");
    n.backspace();
    assert_eq!(n.value(), "JAKE ");
    let mut fb = Framebuffer::new(640, 480);
    n.draw(&mut fb);
    assert_eq!(
        n.handle(Input::Activate, &mut snd),
        Some(Reply::Name("JAKE ".into()))
    );
    assert_eq!(n.handle(Input::Cancel, &mut snd), Some(Reply::Close));
    // texto longo: aparece no campo, recortado nele (nada fora do campo muda)
    let draw = |text: &str, a: &mut Assets| {
        let s = screen(a, "SHIGH.ITF", Placement::Centered);
        let n = NameEntry::new(s, font.clone(), Some("Save Game As:".into()), text);
        let mut fb = Framebuffer::new(640, 480);
        n.draw(&mut fb);
        (fb, n.screen.rect(name_ids::FIELD).unwrap())
    };
    let (empty, r) = draw("", &mut a);
    let (long, _) = draw(&"W".repeat(name_ids::MAX), &mut a);
    let (x0, x1) = (r.x + 4, r.x + r.w - 4);
    let mut inside = 0;
    for y in 0..480 {
        for x in 0..640 {
            let i = y as usize * 640 + x as usize;
            if (x0..x1).contains(&x) && (r.y..r.y + r.h).contains(&y) {
                inside += usize::from(long.pixels[i] == name_ids::COLOR);
            } else {
                assert_eq!(
                    long.pixels[i], empty.pixels[i],
                    "fora do campo em ({x}, {y})"
                );
            }
        }
    }
    assert!(inside > 50, "texto no campo ({inside} pixels)");

    let s = screen(&mut a, "S_CAR.ITF", Placement::Full);
    let mut c = Car::new(s, 7);
    for _ in 0..60 {
        c.handle(Input::Tick, &mut snd);
    }
    let (f, n) = c.screen.frame(car_ids::CAR).unwrap();
    assert!(n > 10 && f == n - 1, "parou no último quadro ({f}/{n})");
    let mut fb = Framebuffer::new(640, 480);
    c.draw(&mut fb);
    assert_eq!(click(&mut c, car_ids::DONE), Some(Reply::Close));

    let s = screen(&mut a, "SMULT.ITF", Placement::Full);
    let mut m = Info::new(s, InfoKind::Multi, None, &[]);
    assert_eq!(
        click(&mut m, info_ids::MULTI_COOP),
        Some(Reply::Unavailable)
    );
    assert_eq!(click(&mut m, info_ids::MULTI_CANCEL), Some(Reply::Close));
    let s = screen(&mut a, "SINPU.ITF", Placement::Full);
    let mut i = openhunted_engine::shell::devices::DeviceScreen::new(s, Default::default(), 0);
    use openhunted_engine::shell::devices::ids as dev;
    assert_eq!(
        i.screen.frame(dev::FIRST[0]).unwrap().0,
        2,
        "teclado marcado"
    );
    assert_eq!(click(&mut i, dev::CUSTOMIZE[0]), Some(Reply::Customize(0)));
    assert_eq!(click(&mut i, dev::DONE), Some(Reply::Close));
    let s = screen(&mut a, "SABOUT.ITF", Placement::Centered);
    let mut ab = Info::new(s, InfoKind::About, Some(font), &["0.1.0".into()]);
    assert_eq!(ab.screen.text(info_ids::ABOUT_VERSION), Some("0.1.0"));
    assert_eq!(click(&mut ab, info_ids::ABOUT_DONE), Some(Reply::Close));
}
