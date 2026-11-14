// SPDX-License-Identifier: GPL-3.0-or-later
//! Telas `.ITF` e barra de estado com os dados reais (OPENHUNTED_DATA).

use openhunted_engine::shell::briefing::{self, Briefing};
use openhunted_engine::shell::hud::{self, Character, HudState};
use openhunted_engine::shell::results::{self, Remains, Results, Stats};
use openhunted_engine::shell::{Input, Placement, Screen, menu};
use openhunted_engine::{Assets, Framebuffer, Outcome};
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

fn rect(s: &Screen, id: u16) -> (i32, i32, i32, i32) {
    let r = s.rect(id).unwrap();
    (r.x, r.y, r.w, r.h)
}

#[test]
fn main_menu_controls_and_frames() {
    let Some(mut a) = assets() else { return };
    let s = screen(&mut a, "SMAIN.ITF", Placement::Full);
    assert_eq!(s.ids().first(), Some(&menu::BACKGROUND));
    assert_eq!(s.ids().len(), 13);
    assert_eq!(rect(&s, menu::NEW_GAME), (270, 64, 219, 32));
    assert_eq!(rect(&s, menu::EXIT), (480, 412, 147, 33));
    assert_eq!(rect(&s, menu::BACKGROUND), (0, 0, 640, 480));
    // botões: normal, em cima, afundando; o fogo anima em laço
    assert_eq!(s.frame(menu::NEW_GAME), Some((0, 3)));
    assert!(s.frame(menu::FIRE).unwrap().1 > 1);
    let mut fb = Framebuffer::new(640, 480);
    s.draw(&mut fb);
    assert!(fb.pixels.iter().filter(|&&p| p == 0).count() < 640 * 480 / 100);
}

#[test]
fn main_menu_new_game_clicks() {
    let Some(mut a) = assets() else { return };
    let mut s = screen(&mut a, "SMAIN.ITF", Placement::Full);
    for id in menu::BUTTONS {
        s.set_flags(id, 0x81);
    }
    s.point_at(menu::NEW_GAME);
    s.handle(Input::Activate);
    let got: Vec<_> = (0..6).filter_map(|_| s.handle(Input::Tick)).collect();
    assert_eq!(got, [menu::NEW_GAME]);
}

#[test]
fn briefing_is_a_centered_dialog() {
    let Some(mut a) = assets() else { return };
    let s = screen(&mut a, "SLEVD.ITF", Placement::Centered);
    let b = s.bounds();
    assert_eq!((b.x, b.y, b.w, b.h), (151, 120, 338, 240));
    assert_eq!(rect(&s, briefing::GO), (151 + 36, 120 + 196, 133, 32));
    assert_eq!(
        rect(&s, briefing::TEXT_AREA),
        (151 + 52, 120 + 59, 238, 115)
    );
    let font = a.font("SLEVD.PFT").unwrap();
    let d = Briefing::new(
        s,
        font.clone(),
        "TUTORIAL: Jake Gets a Gun",
        "Get to the exit.  Use the pistol to shoot the hovering mines.  Good luck Jake!",
    );
    assert!(d.lines().len() > briefing::VISIBLE, "{:?}", d.lines());
    assert_eq!(d.lines()[0], "TUTORIAL: Jake Gets a");
    assert!(d.screen.rect(briefing::UP).is_some(), "setas ligadas");
    // GO com o ponteiro: Enter
    let mut d = d;
    assert_eq!(d.screen.pointer(), (151 + 36 + 66, 120 + 196 + 16));
    d.handle(Input::Activate);
    let got: Vec<_> = (0..6).filter_map(|_| d.handle(Input::Tick)).collect();
    assert_eq!(got, [briefing::Choice::Go]);
    // texto curto: sem setas nem barra
    let s = screen(&mut a, "SLEVD.ITF", Placement::Centered);
    let d = Briefing::new(s, font, "A", "B");
    assert!(d.screen.rect(briefing::UP).is_none());
    assert!(d.screen.rect(briefing::SLIDER).is_none());
}

#[test]
fn briefing_scrolls_with_the_arrows() {
    let Some(mut a) = assets() else { return };
    let s = screen(&mut a, "SLEVD.ITF", Placement::Centered);
    let font = a.font("SLEVD.PFT").unwrap();
    let long = "word ".repeat(80);
    let mut d = Briefing::new(s, font, "T", &long);
    let max = d.lines().len() - briefing::VISIBLE;
    let down = d.screen.rect(briefing::DOWN).unwrap();
    d.handle(Input::PointerMove(down.x + 2, down.y + 2));
    for _ in 0..3 {
        d.handle(Input::Activate);
        for _ in 0..5 {
            d.handle(Input::Tick);
        }
    }
    assert_eq!(d.first_line(), 3.min(max));
    let mut fb = Framebuffer::new(640, 480);
    d.draw(&mut fb);
    assert!(fb.pixels.contains(&briefing::TEXT_COLOR));
}

#[test]
fn results_screen_victory_and_defeat() {
    let Some(mut a) = assets() else { return };
    let font = a.font("SLEVD.PFT").unwrap();
    let s = screen(&mut a, "SMISS.ITF", Placement::Full);
    assert_eq!(rect(&s, results::REPLAY), (34, 397, 189, 33));
    assert_eq!(rect(&s, results::NEXT), (226, 398, 188, 33));
    assert_eq!(rect(&s, results::MAIN), (426, 421, 190, 33));
    assert_eq!(rect(&s, results::SCORE), (381, 255, 176, 26));
    let stats = Stats {
        kills: 2,
        ..Default::default()
    };
    let rem = Remains {
        timer: 0,
        health: 62,
        armor: 6,
        ammo: 80,
    };
    let mut r = Results::new(
        s,
        font.clone(),
        "T",
        Outcome::Victory,
        stats.clone(),
        rem,
        Some(708),
        None,
    );
    assert_eq!(r.score, 60 + 648);
    assert!(r.screen.rect(results::FAILURE).is_none());
    assert!(r.screen.rect(results::SUCCESS).is_some());
    assert_eq!(r.screen.pointer(), (226 + 94, 398 + 16), "cursor em NEXT");
    let mut sounds = Vec::new();
    for _ in 0..30 {
        r.handle(Input::Tick, &mut sounds);
    }
    assert_eq!(
        sounds.len(),
        21,
        "um som por quadro da contagem de 0 a 100 %"
    );
    assert_eq!(r.screen.text(results::SCORE), Some("708"));
    assert_eq!(r.screen.text(results::SCORE_KILLS), Some("60"));
    assert_eq!(r.screen.text(results::PERCENT_KILLS), Some("100"));
    assert_eq!(r.screen.text(results::BONUS), Some("648"));
    assert_eq!(r.handle(Input::Cancel, &mut sounds), None, "Esc ignorado");
    let s = screen(&mut a, "SMISS.ITF", Placement::Full);
    let mut r = Results::new(s, font, "T", Outcome::Defeat, stats, rem, None, None);
    assert_eq!(r.bonus, 0);
    assert!(r.screen.rect(results::SUCCESS).is_none());
    assert_eq!(r.screen.pointer(), (34 + 94, 397 + 16), "cursor em REPLAY");
    r.skip_count();
    let mut sounds = Vec::new();
    r.handle(Input::Activate, &mut sounds);
    let got: Vec<_> = (0..6)
        .filter_map(|_| r.handle(Input::Tick, &mut sounds))
        .collect();
    assert_eq!(got, [results::Choice::Replay]);
    assert_eq!(r.screen.text(results::BONUS), None, "sem bônus na derrota");
}

#[test]
fn hud_bar_frames_match_the_docs() {
    let Some(mut a) = assets() else { return };
    let bar = a.bitmap("INVBAR01.BMP").unwrap();
    assert_eq!(bar.frames.len(), 211);
    let wh = |i: usize| (bar.frames[i].width, bar.frames[i].height);
    assert_eq!(wh(0), (256, 75));
    assert_eq!(wh(1), (321, 32));
    assert_eq!(wh(14), (97, 46));
    assert_eq!(wh(92), (93, 44));
    assert_eq!(wh(193), (18, 34));
    let mut fb = Framebuffer::new(640, 480);
    let mut slots = [(0, 0); 30];
    slots[1] = (1, 20);
    slots[2] = (1, 80);
    let s = HudState {
        character: Character::Jake,
        health: 100,
        armor: 0,
        face_alt: false,
        slots,
        weapon: 2,
        timer: 125 * 20,
        score: None,
    };
    hud::draw_hud(&mut fb, &bar, 0, hud::VIEW_H, 640, &s);
    assert!(
        fb.pixels[..405 * 640].iter().all(|&p| p == 0),
        "só a faixa de baixo"
    );
    assert!(fb.pixels[405 * 640..].iter().filter(|&&p| p == 0).count() < 640 * 75 / 10);
}
