// 侧栏 Agents 列表与窗格边框标题的智能体图标（agent icon）：内置 PNG 资源、Kitty Unicode 占位符编码，
// 以及每个客户端进程对宿主终端的一次性上传。
//
// 图标以 Kitty 图形协议的 Unicode 占位符（U+10EEEE）显示：图片先以虚拟放置上传到宿主终端，
// 之后只需在单元格里写占位符字符，图片随文字一起移动、裁剪和被弹窗覆盖，不需要单独维护坐标。
use std::collections::HashSet;
use std::io::{self, Write};

use ratatui::style::{Color, Style};

use crate::app::state::Palette;
use crate::detect::Agent;
use crate::ui::AgentIconGlyph;

/// 图标占用的终端列数。单元格约为宽 1 高 2，两列一行接近正方形，终端按比例缩放并居中。
pub(crate) const AGENT_ICON_COLUMNS: u16 = 2;

const PLACEHOLDER: char = '\u{10EEEE}';
/// Kitty `rowcolumn-diacritics.txt` 的前两项，分别表示行列编号 0 和 1。
const DIACRITICS: [char; AGENT_ICON_COLUMNS as usize] = ['\u{0305}', '\u{030D}'];

/// 图片 ID 只用前景色的 24 位编码，因此必须小于 2^24；该区间避开 pane 图片使用的
/// 10_000..910_000 与 0x4000_0000 起的 ID 段。
const IMAGE_ID_BASE: u32 = 0x00F1_0000;

/// 图标需要区分深色/浅色主题时选用的变体。单色品牌图标在两种背景下需要不同颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentIconTone {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AgentIconImage {
    id: u32,
    png: &'static [u8],
}

/// 一行智能体在图标位置上显示的内容。没有内置图标的智能体保留同宽空白，让名称对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentIconSlot {
    Image(AgentIconImage),
    Blank,
}

enum IconAsset {
    Single(&'static [u8]),
    Toned {
        dark: &'static [u8],
        light: &'static [u8],
    },
}

macro_rules! icon {
    ($name:literal) => {
        IconAsset::Single(include_bytes!(concat!(
            "../../assets/agent-icons/",
            $name,
            ".png"
        )))
    };
    ($name:literal, toned) => {
        IconAsset::Toned {
            dark: include_bytes!(concat!("../../assets/agent-icons/", $name, "-dark.png")),
            light: include_bytes!(concat!("../../assets/agent-icons/", $name, "-light.png")),
        }
    };
}

fn asset(agent: Agent) -> Option<IconAsset> {
    let asset = match agent {
        Agent::Claude => icon!("claude"),
        Agent::Codex => icon!("codex"),
        Agent::Gemini => icon!("gemini"),
        Agent::Cursor => icon!("cursor", toned),
        Agent::Devin => icon!("devin"),
        Agent::Antigravity => icon!("antigravity"),
        Agent::Cline => icon!("cline", toned),
        Agent::Mastracode => icon!("mastracode", toned),
        Agent::OpenCode => icon!("opencode", toned),
        Agent::GithubCopilot => icon!("copilot", toned),
        Agent::Kimi => icon!("kimi", toned),
        Agent::Kiro => icon!("kiro"),
        Agent::Amp => icon!("amp"),
        Agent::Grok => icon!("grok", toned),
        Agent::Hermes => icon!("hermes", toned),
        Agent::Kilo => icon!("kilo", toned),
        Agent::Qodercli => icon!("qodercli", toned),
        Agent::Qwen => icon!("qwen"),
        Agent::Pi => icon!("pi"),
        Agent::Omp | Agent::Droid | Agent::Letta | Agent::Maki | Agent::Muse => return None,
    };
    Some(asset)
}

impl AgentIconImage {
    pub(crate) fn for_agent(agent: Agent, tone: AgentIconTone) -> Option<Self> {
        let index = Agent::ALL
            .iter()
            .position(|candidate| *candidate == agent)? as u32;
        let (png, tone_bit) = match (asset(agent)?, tone) {
            (IconAsset::Single(png), _) => (png, 0),
            (IconAsset::Toned { dark, .. }, AgentIconTone::Dark) => (dark, 0),
            (IconAsset::Toned { light, .. }, AgentIconTone::Light) => (light, 1),
        };
        Some(Self {
            id: IMAGE_ID_BASE | (index << 1) | tone_bit,
            png,
        })
    }

    /// 占位符靠前景色携带图片 ID；其余文字属性由 Kitty 保留，不应设置。
    fn style(self) -> Style {
        Style::default().fg(self.foreground())
    }

    fn placeholder_text(self) -> String {
        self.placeholder_cells().concat()
    }

    /// 每个终端单元格的占位符字形：行号 0 加列号变音符。
    pub(crate) fn placeholder_cells(self) -> [String; AGENT_ICON_COLUMNS as usize] {
        DIACRITICS.map(|column| [PLACEHOLDER, DIACRITICS[0], column].iter().collect())
    }

    pub(crate) fn foreground(self) -> Color {
        Color::Rgb((self.id >> 16) as u8, (self.id >> 8) as u8, self.id as u8)
    }

    /// 传输 PNG 并创建 `AGENT_ICON_COLUMNS`×1 的虚拟放置；q=2 让终端不回复。
    fn encode_upload(self, out: &mut Vec<u8>) -> io::Result<()> {
        let control = format!(
            "a=T,U=1,f=100,t=d,i={},c={AGENT_ICON_COLUMNS},r=1,q=2",
            self.id
        );
        crate::kitty_graphics::write_kitty_data(out, &control, self.png)
    }
}

impl AgentIconSlot {
    pub(crate) fn for_agent(agent: Option<Agent>, tone: AgentIconTone) -> Self {
        agent
            .and_then(|agent| AgentIconImage::for_agent(agent, tone))
            .map_or(Self::Blank, Self::Image)
    }

    pub(crate) fn image(self) -> Option<AgentIconImage> {
        match self {
            Self::Image(image) => Some(image),
            Self::Blank => None,
        }
    }

    pub(crate) fn glyph(self) -> AgentIconGlyph {
        match self {
            Self::Image(image) => AgentIconGlyph {
                text: image.placeholder_text(),
                style: image.style(),
            },
            Self::Blank => AgentIconGlyph {
                text: " ".repeat(AGENT_ICON_COLUMNS as usize),
                style: Style::default(),
            },
        }
    }
}

/// 按主题文字色判断深浅；文字色不是 RGB（如跟随终端）时退回面板背景色，仍无法判断则按深色处理。
pub(crate) fn tone_for_palette(palette: &Palette) -> AgentIconTone {
    let is_light = |color: Color| match color {
        Color::Rgb(r, g, b) => Some(relative_luminance(r, g, b) > 0.5),
        _ => None,
    };
    if let Some(light_text) = is_light(palette.text) {
        return if light_text {
            AgentIconTone::Dark
        } else {
            AgentIconTone::Light
        };
    }
    match is_light(palette.panel_bg) {
        Some(true) => AgentIconTone::Light,
        _ => AgentIconTone::Dark,
    }
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f32 {
    (0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)) / 255.0
}

/// 宿主终端是否支持 Kitty Unicode 占位符。只认已知支持的终端（kitty、Ghostty）；
/// tmux/screen 会拦截图形转义，WezTerm 等只支持部分 Kitty 图形协议，都不启用。
pub(crate) fn host_supports_placeholders() -> bool {
    use std::io::IsTerminal;
    let env = |key: &str| std::env::var(key).unwrap_or_default();
    io::stdout().is_terminal()
        && placeholder_profile(
            &env("TERM"),
            &env("TERM_PROGRAM"),
            std::env::var_os("KITTY_WINDOW_ID").is_some(),
            std::env::var_os("TMUX").is_some() || std::env::var_os("STY").is_some(),
        )
}

fn placeholder_profile(
    term: &str,
    term_program: &str,
    kitty_window: bool,
    multiplexed: bool,
) -> bool {
    let supported = matches!(term, "xterm-kitty" | "xterm-ghostty")
        || term_program.eq_ignore_ascii_case("ghostty")
        || kitty_window;
    supported && !multiplexed
}

/// 记录本进程已上传到宿主终端的图标。图片在客户端退出时随
/// `frame_output::clear_received_kitty_graphics` 一并删除。
#[derive(Debug, Default)]
pub(crate) struct AgentIconUploads {
    uploaded: HashSet<u32>,
}

impl AgentIconUploads {
    /// 返回尚未上传图标的传输字节和对应 ID；调用方确认写入终端成功后再调用 `mark_uploaded`。
    pub(crate) fn encode_pending(&self, images: &[AgentIconImage]) -> (Vec<u8>, Vec<u32>) {
        let mut bytes = Vec::new();
        let mut ids = Vec::new();
        for image in images {
            if self.uploaded.contains(&image.id) || ids.contains(&image.id) {
                continue;
            }
            if let Err(error) = image.encode_upload(&mut bytes) {
                tracing::warn!(%error, image_id = image.id, "failed to encode agent icon upload");
                continue;
            }
            ids.push(image.id);
        }
        (bytes, ids)
    }

    pub(crate) fn mark_uploaded(&mut self, ids: &[u32]) {
        if !ids.is_empty() {
            tracing::debug!(?ids, "uploaded agent icons to host terminal");
        }
        self.uploaded.extend(ids);
    }
}

pub(crate) fn write_uploads(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    super::frame_output::record_received_kitty_graphics(bytes);
    writer.write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded_payload(bytes: &[u8]) -> Vec<u8> {
        use base64::Engine;
        let text = std::str::from_utf8(bytes).expect("ascii escapes");
        text.split("\x1b_G")
            .filter(|chunk| !chunk.is_empty())
            .flat_map(|chunk| {
                let body = chunk.strip_suffix("\x1b\\").expect("terminated escape");
                let (_, data) = body.split_once(';').expect("payload separator");
                base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .expect("base64 payload")
            })
            .collect()
    }

    #[test]
    fn every_bundled_icon_is_a_png_with_a_unique_24_bit_id() {
        let mut ids = HashSet::new();
        for agent in Agent::ALL {
            for tone in [AgentIconTone::Dark, AgentIconTone::Light] {
                let Some(image) = AgentIconImage::for_agent(agent, tone) else {
                    continue;
                };
                assert!(image.png.starts_with(b"\x89PNG\r\n\x1a\n"), "{agent:?}");
                assert!(image.id < 1 << 24 && image.id >= IMAGE_ID_BASE);
                ids.insert((image.id, image.png.as_ptr()));
            }
        }
        let distinct_ids = ids.iter().map(|(id, _)| *id).collect::<HashSet<_>>();
        assert_eq!(distinct_ids.len(), ids.len(), "one id per distinct asset");
        assert!(AgentIconImage::for_agent(Agent::Claude, AgentIconTone::Dark).is_some());
        assert!(AgentIconImage::for_agent(Agent::Pi, AgentIconTone::Dark).is_some());
        assert!(AgentIconImage::for_agent(Agent::Droid, AgentIconTone::Dark).is_none());
    }

    #[test]
    fn toned_icons_switch_asset_and_id_with_the_theme() {
        let dark = AgentIconImage::for_agent(Agent::Cursor, AgentIconTone::Dark).unwrap();
        let light = AgentIconImage::for_agent(Agent::Cursor, AgentIconTone::Light).unwrap();
        assert_ne!(dark.id, light.id);
        assert_ne!(dark.png, light.png);
        let single_dark = AgentIconImage::for_agent(Agent::Claude, AgentIconTone::Dark).unwrap();
        let single_light = AgentIconImage::for_agent(Agent::Claude, AgentIconTone::Light).unwrap();
        assert_eq!(single_dark, single_light);
    }

    #[test]
    fn placeholder_glyph_encodes_image_id_in_foreground_and_columns_in_diacritics() {
        let image = AgentIconImage::for_agent(Agent::Claude, AgentIconTone::Dark).unwrap();
        let glyph = AgentIconSlot::Image(image).glyph();
        assert_eq!(
            glyph.text,
            "\u{10EEEE}\u{0305}\u{0305}\u{10EEEE}\u{0305}\u{030D}"
        );
        assert_eq!(
            unicode_width::UnicodeWidthStr::width(glyph.text.as_str()),
            AGENT_ICON_COLUMNS as usize
        );
        let Some(Color::Rgb(r, g, b)) = glyph.style.fg else {
            panic!("placeholder needs a 24-bit foreground");
        };
        assert_eq!(u32::from_be_bytes([0, r, g, b]), image.id);
        assert_eq!(glyph.style.bg, None);
        assert!(glyph.style.add_modifier.is_empty());
    }

    #[test]
    fn blank_slot_keeps_icon_width_without_placeholders() {
        let glyph = AgentIconSlot::for_agent(Some(Agent::Droid), AgentIconTone::Dark).glyph();
        assert_eq!(glyph.text, "  ");
        assert_eq!(glyph.style, Style::default());
        assert_eq!(
            AgentIconSlot::for_agent(None, AgentIconTone::Dark),
            AgentIconSlot::Blank
        );
    }

    #[test]
    fn uploads_transmit_png_with_virtual_placement_once() {
        let image = AgentIconImage::for_agent(Agent::Claude, AgentIconTone::Dark).unwrap();
        let mut uploads = AgentIconUploads::default();
        let (bytes, ids) = uploads.encode_pending(&[image, image]);
        assert_eq!(ids, vec![image.id]);
        let header = format!("\x1b_Ga=T,U=1,f=100,t=d,i={},c=2,r=1,q=2,m=", image.id);
        assert!(bytes.starts_with(header.as_bytes()));
        assert_eq!(decoded_payload(&bytes), image.png);

        uploads.mark_uploaded(&ids);
        let (bytes, ids) = uploads.encode_pending(&[image]);
        assert!(bytes.is_empty());
        assert!(ids.is_empty());
    }

    #[test]
    fn tone_follows_palette_text_then_panel_background() {
        let mut palette = Palette::catppuccin();
        palette.text = Color::Rgb(205, 214, 244);
        assert_eq!(tone_for_palette(&palette), AgentIconTone::Dark);
        palette.text = Color::Rgb(76, 79, 105);
        assert_eq!(tone_for_palette(&palette), AgentIconTone::Light);
        palette.text = Color::Reset;
        palette.panel_bg = Color::Rgb(239, 241, 245);
        assert_eq!(tone_for_palette(&palette), AgentIconTone::Light);
        palette.panel_bg = Color::Reset;
        assert_eq!(tone_for_palette(&palette), AgentIconTone::Dark);
    }

    #[test]
    fn placeholder_profile_accepts_kitty_and_ghostty_outside_multiplexers() {
        assert!(placeholder_profile("xterm-kitty", "", false, false));
        assert!(placeholder_profile("xterm-256color", "", true, false));
        assert!(placeholder_profile("xterm-ghostty", "", false, false));
        assert!(placeholder_profile(
            "xterm-256color",
            "ghostty",
            false,
            false
        ));
        assert!(!placeholder_profile("xterm-kitty", "", false, true));
        assert!(!placeholder_profile(
            "xterm-256color",
            "WezTerm",
            false,
            false
        ));
        assert!(!placeholder_profile("xterm-256color", "", false, false));
    }
}
