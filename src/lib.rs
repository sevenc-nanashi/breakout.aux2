use aviutl2::AviUtl2Info;

mod breakout;

#[aviutl2::plugin(GenericPlugin)]
struct BreakoutPlugin {
    thread: Option<breakout::BreakoutHandle>,
}

pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();

impl aviutl2::generic::GenericPlugin for BreakoutPlugin {
    fn new(info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        aviutl2::tracing_subscriber::fmt()
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();
        Ok(Self { thread: None })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "Breakout".to_string(),
            information: "Breakout in AviUtl2!".to_string(),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        registry.register_menus::<Self>();
        EDIT_HANDLE.init(registry.create_edit_handle());
    }
}

impl Drop for BreakoutPlugin {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.stop();
        }
    }
}

#[aviutl2::generic::menus]
impl BreakoutPlugin {
    #[edit(name = "ブロック崩しを開始")]
    fn start_breakout(&mut self) {
        if let Some(thread) = &self.thread
            && thread.is_running()
        {
            tracing::info!("ブロック崩しは既に実行中です");
            return;
        }
        let thread = breakout::BreakoutHandle::new();
        self.thread = Some(thread);
    }
}

aviutl2::register_generic_plugin!(BreakoutPlugin);
