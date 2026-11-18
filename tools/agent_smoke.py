#!/usr/bin/env python3
"""Drive SolidCraft the way an AI agent does, over its MCP server, and check the RulerCraft plugin end to end.

    python3 tools/agent_smoke.py PLUGIN.zip SCAN.stl [path/to/solidcraft-cli]

Installs the plugin, imports the scan as a part, reads the loss report, changes `thickness`, and measures.
Needs a SolidCraft command line that has `mcp --headless`. Prints each step; exits 1 on the first failure.
"""
import json, os, subprocess, sys


class Mcp:
    def __init__(self, cli):
        self.p = subprocess.Popen([cli, "mcp", "--headless", "--allow-any-path"], stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        self.n = 0
        self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                "clientInfo": {"name": "agent_smoke", "version": "0"}})
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
        self.p.stdin.flush()

    def rpc(self, method, params):
        self.n += 1
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.n, "method": method, "params": params}) + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def call(self, tool, args):
        r = self.rpc("tools/call", {"name": tool, "arguments": args})["result"]
        text = r["content"][0]["text"] if r.get("content") else ""
        if r.get("isError"):
            sys.exit(f"{tool} failed: {text[:300]}")
        return json.loads(text)


def main():
    zip_path, scan = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2])
    cli = sys.argv[3] if len(sys.argv) > 3 else "solidcraft-cli"
    m = Mcp(cli)
    m.call("execute", {"command": "plugin.install", "params": {"path": zip_path, "approve": {"memory_mib": 1024}, "replace": True}})
    print("installed the plugin")
    imp = m.call("execute", {"command": "file.import", "params": {"path": scan, "format": "rulercraft.scan-to-part"}})
    report = imp["loss_report"]
    print("imported:", report["summary"])
    for item in report["items"]:
        print(f"  {item['severity']}: {item['title']} ({item['subject']}): {item['message']}")
    check = report["details"].get("check") or sys.exit("the report has no check: is an old plugin installed? " + json.dumps(list(report["details"])))
    print(f"scan vs part: p95 {check['p95_mm']:.3f} mm, worst {check['max_mm']:.3f} mm")
    before = m.call("measure", {})["bodies"][0]["volume_mm3"]
    t = report["details"]["plate"]["thickness"]
    m.call("set_parameter", {"name": "thickness", "value": t + 2})
    after = m.call("measure", {})["bodies"][0]["volume_mm3"]
    print(f"volume {before:.1f} -> {after:.1f} mm3 with thickness {t:g} -> {t + 2:g}")
    if not after > before:
        sys.exit("editing thickness did not change the part")
    print("ok")


main()
