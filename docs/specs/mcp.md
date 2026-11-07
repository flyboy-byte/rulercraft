# MCP server

Status: *Outline*. Feature: `mcp.server`.

A stdio MCP server (`rulercraft mcp`) exposing the command layer as tools, one tool per command, with
the command's JSON schema. Results are compact JSON; large lists are capped and say so. A short
built-in guide tells the agent the usual flow: open, inspect, segment or select, fit, measure, save.
