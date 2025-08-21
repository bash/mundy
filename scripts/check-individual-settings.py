#!/usr/bin/env python3

from subprocess import check_output, check_call
import json, sys
import shlex

metadata = json.loads(
    check_output(["cargo", "metadata", "--format-version", "1", "--no-deps"])
)
package = next((p for p in metadata["packages"] if p["name"] == "mundy"))
preferences = package["features"]["_all-preferences"]

for pref in preferences:
    command = [
        "cargo",
        "clippy",
        "--no-default-features",
        "--features",
        f"async-io,{pref}",
        *sys.argv[1:],
    ]
    pretty_command = shlex.join(command)
    print(f"\x1b[1m{pretty_command}\x1b[0m")
    check_call(command)
