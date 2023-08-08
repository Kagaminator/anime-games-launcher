use relm4::prelude::*;
use relm4::component::*;
use relm4::factory::*;

use gtk::prelude::*;

use crate::components::game_card::{
    GameCardComponent,
    GameCardFactory,
    GameCardComponentInput,
    GameCardComponentOutput
};

use crate::components::game_details::{
    GameDetailsComponent,
    GameDetailsComponentInput,
    GameDetailsComponentOutput
};

use crate::tasks_queue::{
    TasksQueue,
    Task
};

use crate::games::GameVariant;

pub struct MainApp {
    leaflet: adw::Leaflet,
    flap: adw::Flap,

    main_toast_overlay: adw::ToastOverlay,
    game_details_toast_overlay: adw::ToastOverlay,

    game_details: AsyncController<GameDetailsComponent>,
    game_details_variant: GameVariant,

    installed_games: FactoryVecDeque<GameCardFactory>,
    available_games: FactoryVecDeque<GameCardFactory>,

    tasks_queue: TasksQueue,
    downloading_game: AsyncController<GameCardComponent>,
}

#[derive(Debug, Clone)]
pub enum MainAppMsg {
    OpenDetails {
        variant: GameVariant,
        installed: bool
    },

    HideDetails,

    ShowTasksFlap,
    HideTasksFlap,
    ToggleTasksFlap,

    AddTask(Task)
}

#[relm4::component(pub)]
impl SimpleComponent for MainApp {
    type Init = ();
    type Input = MainAppMsg;
    type Output = ();

    view! {
        window = adw::ApplicationWindow {
            set_default_size: (1200, 800),
            set_title: Some("Anime Games Launcher"),

            #[local_ref]
            leaflet -> adw::Leaflet {
                set_can_unfold: false,

                #[local_ref]
                append = main_toast_overlay -> adw::ToastOverlay {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        adw::HeaderBar {
                            add_css_class: "flat",

                            pack_start = &gtk::Button {
                                set_icon_name: "view-dual-symbolic",

                                connect_clicked => MainAppMsg::ToggleTasksFlap
                            }
                        },

                        #[local_ref]
                        flap -> adw::Flap {
                            set_fold_policy: adw::FlapFoldPolicy::Always,
                            set_transition_type: adw::FlapTransitionType::Slide,

                            set_modal: false,

                            #[wrap(Some)]
                            set_flap = &gtk::Box {
                                add_css_class: "background",

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    set_margin_start: 24,
                                    set_margin_end: 24,

                                    model.downloading_game.widget(),

                                    gtk::Label {
                                        set_halign: gtk::Align::Start,

                                        set_margin_top: 24,

                                        add_css_class: "title-4",

                                        #[watch]
                                        set_label: &match model.tasks_queue.get_current() {
                                            Some(task) => format!("Downloading {}", task.get_variant().get_title()),
                                            None => String::from("Nothing to do")
                                        }
                                    },

                                    gtk::ProgressBar {
                                        set_margin_top: 16,
                                        set_fraction: 0.7
                                    },

                                    gtk::Label {
                                        set_halign: gtk::Align::Start,

                                        set_margin_top: 16,

                                        set_label: "Download speed: 20 MB/s"
                                    },

                                    gtk::Label {
                                        set_halign: gtk::Align::Start,

                                        set_margin_top: 8,

                                        set_label: "ETA: 7 minutes"
                                    }
                                }
                            },

                            #[wrap(Some)]
                            set_content = &gtk::ScrolledWindow {
                                set_hexpand: true,
                                set_vexpand: true,
                                
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    gtk::Label {
                                        set_halign: gtk::Align::Start,

                                        set_margin_start: 24,
                                        add_css_class: "title-4",

                                        #[watch]
                                        set_visible: !model.installed_games.is_empty(),

                                        set_label: "Installed games"
                                    },

                                    #[local_ref]
                                    installed_games_flow_box -> gtk::FlowBox {
                                        set_row_spacing: 12,
                                        set_column_spacing: 12,

                                        set_margin_all: 16,

                                        set_homogeneous: true,
                                        set_selection_mode: gtk::SelectionMode::None
                                    },

                                    gtk::Label {
                                        set_halign: gtk::Align::Start,

                                        set_margin_start: 24,
                                        add_css_class: "title-4",

                                        #[watch]
                                        set_visible: !model.available_games.is_empty(),

                                        set_label: "Available games"
                                    },

                                    #[local_ref]
                                    available_games_flow_box -> gtk::FlowBox {
                                        set_row_spacing: 12,
                                        set_column_spacing: 12,

                                        set_margin_all: 16,

                                        set_homogeneous: true,
                                        set_selection_mode: gtk::SelectionMode::None
                                    }
                                }
                            }
                        }
                    }
                },

                #[local_ref]
                append = game_details_toast_overlay -> adw::ToastOverlay {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        #[watch]
                        set_css_classes: &[
                            model.game_details_variant.get_details_style()
                        ],

                        adw::HeaderBar {
                            add_css_class: "flat",

                            pack_start = &gtk::Button {
                                set_icon_name: "go-previous-symbolic",

                                connect_clicked => MainAppMsg::HideDetails
                            }
                        },

                        model.game_details.widget(),
                    }
                }
            }
        }
    }

    fn init(
        _parent: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            leaflet: adw::Leaflet::new(),
            flap: adw::Flap::new(),

            main_toast_overlay: adw::ToastOverlay::new(),
            game_details_toast_overlay: adw::ToastOverlay::new(),

            game_details: GameDetailsComponent::builder()
                .launch(GameVariant::Genshin)
                .forward(sender.input_sender(), |message| match message {
                    GameDetailsComponentOutput::DownloadGame { variant } => {
                        MainAppMsg::AddTask(Task::DownloadGame {
                            variant
                        })
                    }

                    GameDetailsComponentOutput::HideDetails => MainAppMsg::HideDetails,
                    GameDetailsComponentOutput::ShowTasksFlap => MainAppMsg::ShowTasksFlap
                }),

            game_details_variant: GameVariant::Genshin,

            installed_games: FactoryVecDeque::new(gtk::FlowBox::new(), sender.input_sender()),
            available_games: FactoryVecDeque::new(gtk::FlowBox::new(), sender.input_sender()),

            tasks_queue: TasksQueue::new(),

            downloading_game: GameCardComponent::builder()
                .launch(GameVariant::Genshin)
                .detach(),

            // installed_games: vec![
            //     GameCardComponent::builder()
            //         .launch(GameVariant::Genshin)
            //         .detach(),

            //     GameCardComponent::builder()
            //         .launch(GameVariant::Honkai)
            //         .detach()
            // ],

            // available_games: vec![
            //     GameCardComponent::builder()
            //         .launch(GameVariant::StarRail)
            //         .detach()
            // ]
        };

        model.downloading_game.emit(GameCardComponentInput::SetWidth(160));
        model.downloading_game.emit(GameCardComponentInput::SetHeight(224));
        model.downloading_game.emit(GameCardComponentInput::SetClickable(false));
        model.downloading_game.emit(GameCardComponentInput::SetDisplayTitle(false));

        for game in GameVariant::list() {
            let base_folder = game.get_base_installation_folder();

            // match *game {
            //     GameVariant::Genshin => base_folder.push(anime_game_core::game::genshin::Edition::Global.),

            //     _ => ()
            // }

            use anime_game_core::game::GameExt;

            let installed = match *game {
                GameVariant::Genshin => {
                    let game = anime_game_core::game::genshin::Game::new(
                        anime_game_core::filesystem::physical::Driver::new(base_folder),
                        anime_game_core::game::genshin::Edition::Global
                    );

                    true
                    // game.is_installed()
                }

                _ => false
            };

            if installed {
                model.installed_games.guard().push_back(*game);
            }

            else {
                model.available_games.guard().push_back(*game);
            }
        }

        // model.installed_games.guard().push_back(GameVariant::Genshin);
        // model.installed_games.guard().push_back(GameVariant::Honkai);
        // model.installed_games.guard().push_back(GameVariant::PGR);

        // model.available_games.guard().push_back(GameVariant::StarRail);

        model.available_games.broadcast(GameCardComponentInput::SetInstalled(false));

        let leaflet = &model.leaflet;
        let flap = &model.flap;

        let main_toast_overlay = &model.main_toast_overlay;
        let game_details_toast_overlay = &model.game_details_toast_overlay;

        let installed_games_flow_box = model.installed_games.widget();
        let available_games_flow_box = model.available_games.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            MainAppMsg::OpenDetails { variant, installed } => {
                self.game_details_variant = variant;

                self.game_details.emit(GameDetailsComponentInput::SetVariant(variant));
                self.game_details.emit(GameDetailsComponentInput::SetInstalled(installed));

                self.leaflet.navigate(adw::NavigationDirection::Forward);
            }

            MainAppMsg::HideDetails => {
                self.leaflet.navigate(adw::NavigationDirection::Back);
            }

            MainAppMsg::ShowTasksFlap => {
                self.flap.set_reveal_flap(true);
            }

            MainAppMsg::HideTasksFlap => {
                self.flap.set_reveal_flap(false);
            }

            MainAppMsg::ToggleTasksFlap => {
                self.flap.set_reveal_flap(!self.flap.reveals_flap());
            }

            MainAppMsg::AddTask(task) => {
                self.tasks_queue.push(task);
            }
        }
    }
}