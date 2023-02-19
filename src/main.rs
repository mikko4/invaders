use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use invaders::{
    frame::{self, new_frame, Drawable},
    invaders::Invaders,
    player::Player,
    render,
};
use rusty_audio::Audio;
use std::{
    error::Error,
    io::{self, Stdout},
    sync::mpsc,
    thread::{self, sleep},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn Error>> {
    /* SET UP */
    // audio
    let mut audio: Audio = Audio::new();
    audio.add("explode", "sounds/explode.wav");
    audio.add("lose", "sounds/samsung.mp3");
    audio.add("move", "sounds/move.wav");
    audio.add("pew", "sounds/pew.wav");
    audio.add("startup", "sounds/aveIt.wav");
    audio.add("win", "sounds/ohYes.wav");
    audio.play("startup");

    // terminal
    let mut stdout: Stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;

    // render loop in separate thread
    let (render_tx, render_rx) = mpsc::channel();
    let render_handle = thread::spawn(move || {
        let mut last_frame = frame::new_frame();
        let mut stdout = io::stdout();
        render::render(&mut stdout, &last_frame, &last_frame, true);
        loop {
            let curr_frame = match render_rx.recv() {
                Ok(x) => x,
                Err(_) => break,
            };
            render::render(&mut stdout, &last_frame, &curr_frame, false);
            last_frame = curr_frame;
        }
    });

    /* GAME */
    let mut player = Player::new();
    let mut instant = Instant::now();
    let mut invaders = Invaders::new();
    'gameloop: loop {
        // per-frame init
        let delta = instant.elapsed();
        instant = Instant::now();
        let mut curr_frame = new_frame();

        // input
        while event::poll(Duration::default())? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Left => player.move_left(),
                    KeyCode::Right => player.move_right(),
                    KeyCode::Down => player.move_down(),
                    KeyCode::Up => player.move_up(),
                    KeyCode::Char(' ') => {
                        if player.shoot() {
                            audio.play("pew");
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        audio.play("lose");
                        sleep(Duration::from_millis(1600));
                        break 'gameloop;
                    }
                    _ => {}
                }
            }
        }

        // updates
        player.update(delta);
        if invaders.update(delta) {
            audio.play("move")
        }
        if player.detect_hits(&mut invaders) {
            audio.play("explode");
        }

        // draw and render
        let drawables: Vec<&dyn Drawable> = vec![&player, &invaders];
        for drawable in drawables {
            drawable.draw(&mut curr_frame);
        }
        let _ = render_tx.send(curr_frame);
        thread::sleep(Duration::from_millis(1));

        // win or lose the game
        if invaders.all_dead() {
            audio.play("win");
            sleep(Duration::from_millis(2400));
            break 'gameloop;
        }
        if invaders.reached_bottom() {
            audio.play("lose");
            sleep(Duration::from_millis(1600));
            break 'gameloop;
        }
    }

    /* CLEAN UP */
    drop(render_tx);
    render_handle.join().unwrap();
    audio.wait();
    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
