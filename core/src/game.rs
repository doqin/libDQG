use winit::event_loop::EventLoop;
use crate::scene::Scene;

use crate::app::App;

pub struct Game {
    app: App<'static>,
}

impl Game {
    pub fn new(app: App<'static>) -> Self {
        Self {
            app,
        }
    }

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        match event_loop.run_app(&mut self.app) {
            Ok(_) => println!("Application exited successfully."),
            Err(e) => eprintln!("Application error: {:?}", e),
        }
    }
}

pub struct GameBuilder {
    title: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    scenes: Vec<Box<dyn Scene>>,
}

impl GameBuilder {
    pub fn new(initial_scene: Box<dyn Scene>) -> Self {
        Self {
            title: None,
            width: None,
            height: None,
            scenes: vec![initial_scene],
        }
    }

    pub fn title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn add_scene(mut self, scene: Box<dyn Scene>) -> Self {
        self.scenes.push(scene);
        self
    }

    pub fn build(self) -> Game {
        let title = self.title.unwrap_or_else(|| "libDQG Game".to_string());
        let width = self.width.unwrap_or(800);
        let height = self.height.unwrap_or(600);

        let mut app = App::new(title, width, height);
        for scene in self.scenes {
            app.add_scene(scene);
        }
        Game::new(app)
    }
}