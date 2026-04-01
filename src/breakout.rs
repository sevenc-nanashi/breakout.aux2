use crate::EDIT_HANDLE;

pub struct BreakoutHandle {
    thread: std::thread::JoinHandle<()>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl BreakoutHandle {
    pub fn new() -> Self {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_clone = stop.clone();
        let thread = std::thread::spawn(move || {
            BreakoutGame::run(stop_clone);
        });
        Self { thread, stop }
    }
    pub fn is_running(&self) -> bool {
        !self.stop.load(std::sync::atomic::Ordering::Relaxed) && !self.thread.is_finished()
    }

    pub fn stop(self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        self.thread.join().unwrap();
    }
}

struct BreakoutGame {
    ball_x_frame: i64,
    ball_layer: i64,
    max_frame: u64,
    bar_layer: u64,
    velocity_x: i64,
    velocity_y: i64,
    ball_object: aviutl2::generic::ObjectHandle,
    bar_object: aviutl2::generic::ObjectHandle,
    bar_x_frame: i64,
    bar_max_frame: u64,
}

const UNICODE_BLOCK: char = '\u{2588}';
const PADDING_AREA: u64 = 4;

const UNIT_SIZE: u64 = 5;
const BAR_WIDTH: u64 = UNIT_SIZE * 5;

impl BreakoutGame {
    pub fn new() -> anyhow::Result<Self> {
        let info = EDIT_HANDLE.get_edit_info();
        if info.frame_max == 0 {
            anyhow::bail!("オブジェクトが存在しません");
        }
        tracing::info!(
            "フレーム数: {}, レイヤー数: {}",
            info.frame_max,
            info.layer_max
        );
        let ball_layer = (info.layer_max as u64 + PADDING_AREA) as i64;
        let ball_x_frame = (info.frame_max as i64) / 2;
        let bar_x_frame = (info.frame_max as i64) / 2 - (BAR_WIDTH as i64) / 2;
        let (ball, bar) = EDIT_HANDLE.call_edit_section(|edit| {
            let ball = edit.create_object(
                "テキスト",
                ball_layer as _,
                ball_x_frame as _,
                Some(UNIT_SIZE as _),
            )?;
            edit.set_object_effect_item(
                &ball,
                "テキスト",
                0,
                "テキスト",
                &UNICODE_BLOCK.to_string(),
            )?;

            let bar = edit.create_object(
                "テキスト",
                (info.layer_max as u64 + PADDING_AREA + 1) as _,
                bar_x_frame as _,
                Some(BAR_WIDTH as _),
            )?;
            edit.set_object_effect_item(
                &bar,
                "テキスト",
                0,
                "テキスト",
                &UNICODE_BLOCK.to_string().repeat(5),
            )?;
            anyhow::Ok((ball, bar))
        })??;
        Ok(Self {
            ball_x_frame,
            ball_layer,
            velocity_x: (UNIT_SIZE * 2) as i64,
            velocity_y: -1,
            max_frame: info.frame_max as u64 - UNIT_SIZE,
            bar_layer: (info.layer_max as u64 + PADDING_AREA + 1),
            ball_object: ball,
            bar_object: bar,
            bar_x_frame,
            bar_max_frame: info.frame_max as u64 - (UNIT_SIZE * 5),
        })
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        let mut ball_x_frame = self.ball_x_frame + self.velocity_x;

        if ball_x_frame < 0 || ball_x_frame >= self.max_frame as i64 {
            self.velocity_x = -self.velocity_x;
            ball_x_frame = ball_x_frame.clamp(0, self.max_frame as i64 - 1);
        }
        self.ball_x_frame = ball_x_frame;
        if self.ball_layer <= 0 {
            self.velocity_y = -self.velocity_y;
            self.ball_layer = 1;
        }
        if self.ball_layer >= self.bar_layer as i64 {
            anyhow::bail!("ゲームオーバー");
        }
        self.ball_layer += self.velocity_y;

        EDIT_HANDLE.call_edit_section(|edit| {
            let target_bar_x = edit.info.frame as i64 - BAR_WIDTH as i64 / 2;
            self.bar_x_frame = target_bar_x.clamp(0, self.bar_max_frame as i64);
            edit.move_object(&self.bar_object, self.bar_layer as _, self.bar_x_frame as _)?;

            tracing::info!(
                "ball_x_frame: {}, ball_layer: {}",
                self.ball_x_frame,
                self.ball_layer
            );
            if self.bar_layer as i64 == self.ball_layer
                && ((self.bar_x_frame..self.bar_x_frame + (BAR_WIDTH as i64))
                    .contains(&self.ball_x_frame)
                    || (self.bar_x_frame..self.bar_x_frame + (BAR_WIDTH as i64))
                        .contains(&(self.ball_x_frame + UNIT_SIZE as i64 - 1)))
            {
                self.velocity_y = -self.velocity_y;
                self.ball_layer -= 2;
            }
            let hit_object =
                edit.objects_in_layer(self.ball_layer as _)
                    .find(|(position, _handle)| {
                        (position.start..=position.end).contains(&(self.ball_x_frame as _))
                    });
            if let Some((position, handle)) = hit_object {
                tracing::info!(
                    "Hit object at layer {}, frame {}-{}",
                    self.ball_layer,
                    position.start,
                    position.end
                );
                self.velocity_y = -self.velocity_y;
                self.ball_layer += self.velocity_y;
                edit.delete_object(&handle)?;
            }
            edit.move_object(
                &self.ball_object,
                self.ball_layer as _,
                self.ball_x_frame as _,
            )?;

            anyhow::Ok(())
        })??;

        Ok(())
    }

    pub fn run(terminate_signal: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        let mut game = match Self::new() {
            Ok(game) => game,
            Err(e) => {
                tracing::error!("ゲームの初期化に失敗しました: {}", e);
                return;
            }
        };

        while !terminate_signal.load(std::sync::atomic::Ordering::Relaxed) {
            if let Err(e) = game.update() {
                tracing::error!("ゲームの更新に失敗しました: {}", e);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}
