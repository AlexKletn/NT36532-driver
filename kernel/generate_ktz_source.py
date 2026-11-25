# SPDX-License-Identifier: GPL-2.0-only

import json
import sys
from pathlib import Path

checkout = Path(sys.argv[1]).resolve()
module = checkout / "crates/ktz8866-driver/src/chip/mod.rs"
if not module.is_file():
    sys.exit(f"{module}: KTZ8866 driver source was not found")

print(f"#[path = {json.dumps(str(module), ensure_ascii=False)}]")
print("#[allow(dead_code, unreachable_pub)]")
print("mod ktz_chip;")
print("pub(crate) use ktz_chip::{BacklightConfig, BiasConfig, Delay, DriverError, Ktz8866, RegisterBus};")
