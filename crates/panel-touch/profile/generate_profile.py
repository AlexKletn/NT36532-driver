import re
import sys
import tomllib
from pathlib import Path


def number(value, minimum, maximum):
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError(f"expected an integer in {minimum}..={maximum}, got {value!r}")
    return value


def pair(value, minimum, maximum):
    if not isinstance(value, list) or len(value) != 2:
        raise ValueError("expected an array with two values")
    first, second = (number(item, minimum, maximum) for item in value)
    return f"[{first}, {second}]"


def wire_max(profile, key):
    if key not in profile:
        return "None"
    return f"Some({pair(profile[key], 1, 65535)})"


def generate(profile, crc):
    formats = {"packed12-v1": "Packed12", "big-endian16-v1": "BigEndian16"}
    name = profile["firmware_name"]
    if not re.fullmatch(r"[A-Za-z0-9_-]+(?:/[A-Za-z0-9_-]+)*\.bin", name):
        raise ValueError("expected a relative .bin firmware name")
    contact_format = formats[profile["strategy"]]
    contact_max = wire_max(profile, "contact_wire_max")
    pen_max = wire_max(profile, "pen_wire_max")
    masks = pair(profile["pen_button_masks"], 0, 255)
    return f"""FirmwareProfile {{
    firmware_name: "{name}",
    crc: {list(crc)},
    contact_format: super::ContactFormat::{contact_format},
    contact_wire_max: {contact_max},
    pen_wire_max: {pen_max},
    pen_button_masks: {masks},
}},
"""


directory, destination = map(Path, sys.argv[1:])
try:
    if not directory.is_dir():
        raise ValueError("expected a touch profile directory")
    sources = sorted(directory.glob("*.toml"))
    if not sources:
        raise ValueError("no TOML profiles found")
except (OSError, ValueError) as error:
    sys.exit(f"{directory}: {error}")

profiles = {}
for source in sources:
    try:
        with source.open("rb") as file:
            profile = tomllib.load(file)
        crc = tuple(number(profile[key], 0, 0xFFFFFFFF) for key in ("ilm_crc", "dlm_crc"))
        if crc in profiles:
            raise ValueError(f"duplicate ILM/DLM CRC pair already defined in {profiles[crc][0]}")
        profiles[crc] = source, generate(profile, crc)
    except (OSError, ValueError, KeyError, TypeError) as error:
        sys.exit(f"{source}: {error}")

output = "pub static TOUCH_PROFILES: &[FirmwareProfile] = &[\n"
output += "".join(entry for _, entry in profiles.values())
destination.write_text(output + "];\n", encoding="utf-8")
