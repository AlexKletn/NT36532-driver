# SPDX-License-Identifier: GPL-2.0-only

from pathlib import Path
import sys
import tomllib


def integer(value, name, minimum=0, maximum=65535):
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError(f"{name} must be an integer from {minimum} to {maximum}")
    return value


def render_commands(group, name, table):
    mode = {"lp": "LowPower", "hs": "HighSpeed"}[table["link_mode"]]
    lines = [f"const {name}: &[DsiCommand<'static>] = &["]
    for index, command in enumerate(table["packets"]):
        label = f"{group}.packets[{index}]"
        payload = [integer(value, f"{label}.payload", maximum=255)
                   for value in command["payload"]]
        match command["type"], len(payload):
            case "short", 1:
                packet = f"DcsShort(0x{payload[0]:02x})"
            case "long", length if length >= 2:
                values = ", ".join(f"0x{value:02x}" for value in payload)
                packet = f"DcsLong(&[{values}])"
            case _:
                raise ValueError(f"{label} has an invalid packet type or length")
        delay = integer(command.get("delay_ms", 0), f"{label}.delay_ms")
        lines.append(
            f"    DsiCommand {{ packet: DsiPacket::{packet}, "
            f"mode: DsiMode::{mode}, delay_after_ms: {delay} }},"
        )
    return lines + ["];"]


def render_panel(panel):
    delay = integer(panel["vddio_settle_ms"], "panel.vddio_settle_ms")
    lines = ["const PANEL: PanelProfile<'static> = PanelProfile {",
             f"    vddio_settle_ms: {delay},", "    reset: &["]
    for index, step in enumerate(panel["reset"]):
        if type(step["asserted"]) is not bool:
            raise ValueError(f"panel.reset[{index}].asserted must be a boolean")
        asserted = str(step["asserted"]).lower()
        delay = integer(step["delay_ms"], f"panel.reset[{index}].delay_ms")
        lines.append(f"        ResetStep {{ asserted: {asserted}, delay_after_ms: {delay} }},")
    return lines + ["    ],", "    init: INIT,", "    shutdown: SHUTDOWN,", "};"]


def render_modes(panel, modes, names):
    fields = ("hdisplay", "hsync", "hback", "vdisplay", "vsync", "vback")
    shared = {key: integer(panel[key], f"panel.{key}") for key in fields}
    lines = ["pub(crate) const MODES: &[DisplayMode] = &["]
    seen = set()
    for index, mode in enumerate(modes):
        label = f"modes[{index}]"
        hz = integer(mode["refresh_hz"], f"{label}.refresh_hz", minimum=1)
        if hz in seen:
            raise ValueError(f"duplicate refresh rate: {hz} Hz")
        seen.add(hz)
        front = {key: integer(mode[key], f"{label}.{key}") for key in ("hfront", "vfront")}
        lines.append("    DisplayMode {")
        for key, value in {"refresh_hz": hz, **shared, **front}.items():
            lines.append(f"        {key}: {value},")
        lines += [f"        post_init: {names[mode['post_init']]},", "    },"]
    if not seen:
        raise ValueError("at least one display mode is required")
    return lines + ["];"]


def render_link(link):
    if link is None:
        return []
    ranges = {
        "slice_width": (1, 65535), "slice_height": (1, 65535),
        "bits_per_pixel": (1, 1023), "slices_per_channel": (1, 255),
        "bits_per_component": (8, 16), "lanes": (1, 4),
        "version_major": (1, 1), "version_minor": (1, 2),
    }
    lines = ["use crate::PanelLinkConfig;", "", "const LINK: PanelLinkConfig = PanelLinkConfig {"]
    for key, (low, high) in ranges.items():
        lines.append(f"    {key}: {integer(link[key], f'dsi.{key}', low, high)},")
    for key in ("block_prediction", "burst_mode", "noncontinuous_clock"):
        value = link[key]
        if type(value) is not bool:
            raise ValueError(f"dsi.{key} must be a boolean")
        lines.append(f"    {key}: {str(value).lower()},")
    return lines + ["};", ""]


def render_ktz8866(config):
    if config is None:
        return []
    lines = ["use crate::Ktz8866Profile;", "", "const BACKLIGHT: Ktz8866Profile = Ktz8866Profile {"]
    ranges = {"sink_mask": (1, 63)}
    for key in ("led_config_1", "led_config_2", "dimming", "led_option_2", "pwm_ramp",
                "boost", "positive_bias", "negative_bias"):
        ranges[key] = (0, 255)
    for key in ("bias_step_ms", "settle_on_ms", "settle_off_ms"):
        ranges[key] = (1, 65535)
    for key, (low, high) in ranges.items():
        lines.append(f"    {key}: {integer(config[key], f'ktz8866.{key}', low, high)},")
    return lines + ["};", ""]


def render_dsi_command_sequence(source):
    panel = source["panel"]
    groups = source["commands"]
    link = source.get("dsi")
    backlight = source.get("ktz8866")
    names = {group: f"COMMANDS_{index}" for index, group in enumerate(groups)}
    names.update(init="INIT", shutdown="SHUTDOWN")
    lines = ["use crate::{DsiCommand, DsiMode, DsiPacket, DisplayMode, PanelProfile, ResetStep, SelectedProfile};", ""]
    lines += render_link(link)
    lines += render_ktz8866(backlight)
    for group, name in names.items():
        lines += render_commands(group, name, groups[group]) + [""]
    lines += render_panel(panel) + [""]
    modes = source["modes"]
    lines += render_modes(panel, modes, names)
    preferred = integer(panel.get("preferred_refresh_hz", modes[0]["refresh_hz"]),
                        "panel.preferred_refresh_hz", minimum=1)
    if not any(mode["refresh_hz"] == preferred for mode in modes):
        raise ValueError("preferred refresh rate must match a declared mode")
    lines += [
        "",
        "pub(super) const PROFILE: SelectedProfile = SelectedProfile {",
        "    panel: PANEL,",
        f"    link: {'Some(LINK)' if link is not None else 'None'},",
        f"    ktz8866: {'Some(BACKLIGHT)' if backlight is not None else 'None'},",
        "    modes: MODES,",
        f"    preferred_refresh_hz: {preferred},",
        "};",
    ]
    module = ["// Generated from the selected panel profile.", "", "mod dsi_command_sequence {"]
    module += [f"    {line}" if line else "" for line in lines]
    module += ["}", "", "pub static SELECTED_PROFILE: Option<SelectedProfile> = Some(dsi_command_sequence::PROFILE);"]
    return "\n".join(module) + "\n"


def generate(source):
    strategy = source["strategy"]
    if strategy == "dsi-command-sequence-v1":
        return render_dsi_command_sequence(source)
    raise ValueError(f"unsupported panel profile strategy: {strategy!r}")


if __name__ == "__main__":
    source, output = map(Path, sys.argv[1:])
    try:
        with source.open("rb") as profile:
            rendered = generate(tomllib.load(profile))
        output.write_text(rendered, encoding="utf-8")
    except (KeyError, TypeError, ValueError) as error:
        sys.exit(f"{source}: {error}")
