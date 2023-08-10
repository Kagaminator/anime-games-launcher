use std::collections::HashMap;

use relm4::prelude::*;
use relm4::component::*;
use relm4::factory::*;

use gtk::prelude::*;
use adw::prelude::*;

use anime_game_core::game::GameExt;
use anime_game_core::game::diff::GetDiffExt;
use anime_game_core::game::diff::DiffExt;

use crate::ui::components::game_card::{
    GameCardComponentInput,
    CardVariant
};

use crate::ui::components::factory::game_card_main::GameCardFactory;

use crate::ui::components::game_details::{
    GameDetailsComponent,
    GameDetailsComponentInput,
    GameDetailsComponentOutput
};

use crate::ui::components::tasks_queue::{
    TasksQueueComponent,
    TasksQueueComponentInput,
    TasksQueueComponentOutput,
    Task
};

use crate::config;

static mut MAIN_WINDOW: Option<adw::ApplicationWindow> = None;

pub struct MainApp {
    leaflet: adw::Leaflet,
    flap: adw::Flap,

    main_toast_overlay: adw::ToastOverlay,
    game_details_toast_overlay: adw::ToastOverlay,

    game_details: AsyncController<GameDetailsComponent>,
    game_details_variant: CardVariant,

    installed_games: FactoryVecDeque<GameCardFactory>,
    queued_games: FactoryVecDeque<GameCardFactory>,
    available_games: FactoryVecDeque<GameCardFactory>,

    installed_games_indexes: HashMap<CardVariant, DynamicIndex>,
    queued_games_indexes: HashMap<CardVariant, DynamicIndex>,
    available_games_indexes: HashMap<CardVariant, DynamicIndex>,

    tasks_queue: AsyncController<TasksQueueComponent>
}

#[derive(Debug, Clone)]
pub enum MainAppMsg {
    OpenDetails {
        variant: CardVariant,
        installed: bool
    },

    HideDetails,

    ShowTasksFlap,
    HideTasksFlap,
    ToggleTasksFlap,

    AddDownloadGameTask(CardVariant),

    ShowTitle {
        title: String,
        message: Option<String>
    }
}

#[relm4::component(pub)]
impl SimpleComponent for MainApp {
    type Init = ();
    type Input = MainAppMsg;
    type Output = ();

    view! {
        main_window = adw::ApplicationWindow {
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

                                model.tasks_queue.widget(),
                            },

                            #[wrap(Some)]
                            set_separator = &gtk::Separator,

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
                                        set_visible: !model.queued_games.is_empty(),

                                        set_label: "Queued games"
                                    },

                                    #[local_ref]
                                    queued_games_flow_box -> gtk::FlowBox {
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
                .launch(CardVariant::Genshin)
                .forward(sender.input_sender(), |message| match message {
                    GameDetailsComponentOutput::DownloadGame { variant } => {
                        MainAppMsg::AddDownloadGameTask(variant)
                    }

                    GameDetailsComponentOutput::HideDetails => MainAppMsg::HideDetails,
                    GameDetailsComponentOutput::ShowTasksFlap => MainAppMsg::ShowTasksFlap
                }),

            game_details_variant: CardVariant::Genshin,

            installed_games_indexes: HashMap::new(),
            queued_games_indexes: HashMap::new(),
            available_games_indexes: HashMap::new(),

            installed_games: FactoryVecDeque::new(gtk::FlowBox::new(), sender.input_sender()),
            queued_games: FactoryVecDeque::new(gtk::FlowBox::new(), sender.input_sender()),
            available_games: FactoryVecDeque::new(gtk::FlowBox::new(), sender.input_sender()),

            tasks_queue: TasksQueueComponent::builder()
                .launch(CardVariant::Genshin)
                .forward(sender.input_sender(), |output| match output {
                    TasksQueueComponentOutput::ShowToast { title, message } => MainAppMsg::ShowTitle { title, message }
                }),
        };

        let config = config::get();

        for game in CardVariant::games() {
            let installed = match *game {
                CardVariant::Genshin => config.games.genshin.to_game().is_installed(),

                _ => false
            };

            if installed {
                model.installed_games_indexes.insert(*game, model.installed_games.guard().push_back(*game));
            }

            else {
                model.available_games_indexes.insert(*game, model.available_games.guard().push_back(*game));
            }
        }

        model.available_games.broadcast(GameCardComponentInput::SetInstalled(false));

        let leaflet = &model.leaflet;
        let flap = &model.flap;

        let main_toast_overlay = &model.main_toast_overlay;
        let game_details_toast_overlay = &model.game_details_toast_overlay;

        let installed_games_flow_box = model.installed_games.widget();
        let queued_games_flow_box = model.queued_games.widget();
        let available_games_flow_box = model.available_games.widget();

        let widgets = view_output!();

        unsafe {
            MAIN_WINDOW = Some(widgets.main_window.clone());
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
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

            MainAppMsg::AddDownloadGameTask(variant) => {
                let config = config::get();

                let task = match variant {
                    CardVariant::Genshin => {
                        Task::DownloadGenshinDiff {
                            updater: config.games.genshin
                                .to_game()
                                .get_diff()
                                .unwrap() // FIXME
                                .install()
                                .unwrap() // FIXME
                        }
                    },

                    _ => unimplemented!()
                };

                self.tasks_queue.emit(TasksQueueComponentInput::AddTask(task));

                if let Some(index) = self.available_games_indexes.get(&variant) {
                    self.available_games.guard().remove(index.current_index());

                    self.available_games_indexes.remove(&variant);

                    #[allow(clippy::map_entry)]
                    if !self.queued_games_indexes.contains_key(&variant) {
                        self.queued_games_indexes.insert(variant, self.queued_games.guard().push_back(variant));

                        self.queued_games.broadcast(GameCardComponentInput::SetInstalled(false));
                        self.queued_games.broadcast(GameCardComponentInput::SetClickable(false));
                    }
                }
            }

            MainAppMsg::ShowTitle { title, message } => {
                let toast = adw::Toast::new(&title);

                // toast.set_timeout(7);

                if let Some(message) = message {
                    toast.set_button_label(Some("Details"));

                    let dialog = adw::MessageDialog::new(
                        Some(unsafe { MAIN_WINDOW.as_ref().unwrap_unchecked() }),
                        Some(&title),
                        Some(&message)
                    );

                    dialog.add_response("close", "Close");
                    // dialog.add_response("save", &tr!("save"));

                    // dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);

                    // dialog.connect_response(Some("save"), |_, _| {
                    //     if let Err(err) = open::that(crate::DEBUG_FILE.as_os_str()) {
                    //         tracing::error!("Failed to open debug file: {err}");
                    //     }
                    // });

                    toast.connect_button_clicked(move |_| {
                        dialog.present();
                    });
                }

                self.main_toast_overlay.add_toast(toast);
            }
        }
    }
}