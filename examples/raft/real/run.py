import os
import sys
from pathlib import Path

if len(sys.argv) < 3:
    print(f'Usage: ./{sys.argv[0]} <config_name> <replica>')
    exit(1)

raft_bin = "RAFT_BIN"

bin_path: str

if raft_bin not in os.environ:
    print(f'The environment variable "{raft_bin}" not specified')
    bin_path = Path(os.path.dirname(__file__) + "../../../../target/debug/run-raft-real").resolve()
    print(f'Setting binary path to \"{bin_path}\"')
else:
    bin_path = Path(os.environ[raft_bin]).resolve()

config_name = sys.argv[1]
config_path = f"cfg/{config_name}.json"
config_path = Path(config_path).resolve()

replica = int(sys.argv[2])

storage_path = f".system/{config_name}/{replica}"
storage_path = Path(storage_path).resolve()

Path(storage_path).mkdir(parents=True, exist_ok=True)

print("\n...Running...\n")

os.system(f"{bin_path} {config_path} {replica} {storage_path}")