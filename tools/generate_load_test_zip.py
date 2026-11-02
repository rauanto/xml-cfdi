#!/usr/bin/env python3
"""Create a ZIP with repeated valid CFDI fixture entries for UI load testing."""

import argparse
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "cfdi40_ganado_iva_tasa0.xml"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("destination", type=Path, help="output ZIP path")
    parser.add_argument("--count", type=int, default=5000, help="number of XML entries")
    args = parser.parse_args()
    if args.count < 1:
        parser.error("--count must be at least 1")

    fixture = FIXTURE.read_bytes()
    args.destination.parent.mkdir(parents=True, exist_ok=True)
    with ZipFile(args.destination, "w", compression=ZIP_DEFLATED) as archive:
        for index in range(args.count):
            archive.writestr(f"sinteticos/cfdi_{index:05}.xml", fixture)
    print(f"Generated {args.count} XML files: {args.destination}")


if __name__ == "__main__":
    main()
