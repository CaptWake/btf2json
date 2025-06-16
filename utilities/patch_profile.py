"""
This script will add the following type definition to be able to use the generated profile with volatility3
"long unsigned int": {
      "size": 8,
      "signed": false,
      "kind": "int",
      "endian": "little"
    },
"""

import argparse
import json


def patch_profile(profile_fpath: str):

    print(f"[+] Patching {profile_fpath}...")

    with open(profile_fpath, "r") as file:
        profile_content = json.load(file)

    profile_content["base_types"]["long unsigned int"] = {
        "size": 8,
        "signed": False,
        "kind": "int",
        "endian": "little",
    }

    with open(profile_fpath, "w") as file:
        json.dump(profile_content, file)

    print(f"[+] Successfully patched {profile_fpath}.")


if __name__ == "__main__":
    argparser = argparse.ArgumentParser(
        description="Patch the btf2json volatility profile."
    )
    argparser.add_argument(
        "-f",
        type=str,
        required=True,
        help="Path to the btf2json vol profile to be patched.",
    )

    args = argparser.parse_args()
    patch_profile(args.f)
