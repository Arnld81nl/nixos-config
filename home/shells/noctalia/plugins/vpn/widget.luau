-- Bar widget showing which VPNs are currently connected.
--
-- All the work is in ~/.local/bin/vpn-status (home/home.nix); this only polls
-- its `--bar` JSON and paints it:
--
--   {"text": "VPN" | "<name> <name>", "glyph": "shield"|"shield-check",
--    "tooltip": "● name — connected\n○ name — off", "textColor": "..."}
--
-- Left click forces an immediate re-read, which is what you want right after
-- toggling one of the buttons the accordion reveals.

local SCRIPT = noctalia.expandPath("~/.local/bin/vpn-status")

-- The checks are local (pgrep plus `sudo -n swanctl --list-sas`), so polling is
-- cheap; 5s keeps the pill in step with a toggle without visible lag.
local POLL_MS = 5000

-- Nothing here talks to the network, so a short timeout is enough — and a stuck
-- swanctl should surface as an error rather than freeze the widget.
local TIMEOUT_MS = 5000

noctalia.setUpdateInterval(POLL_MS)

-- Every field is set on every paint: a patch that omits one leaves the previous
-- value on screen, so an error state would otherwise keep the last good color.
local function render(glyph, text, tooltip, color)
  barWidget.setGlyph(glyph)
  barWidget.setText(text)
  barWidget.setColor(color)
  barWidget.setGlyphColor(color)
  if tooltip ~= "" then
    barWidget.setTooltip(tooltip)
  else
    barWidget.clearTooltip()
  end
end

local function apply(result)
  if result.timedOut then
    render("shield-x", "VPN", "vpn-status timed out", "error")
    return
  end
  if result.exitCode ~= 0 then
    render("shield-x", "VPN", "vpn-status exited " .. tostring(result.exitCode) .. "\n" .. result.stderr, "error")
    return
  end

  local payload = noctalia.json.decode(result.stdout)
  if type(payload) ~= "table" or type(payload.text) ~= "string" then
    render("shield-x", "VPN", "vpn-status returned no usable JSON", "error")
    return
  end

  render(
    payload.glyph or "shield",
    payload.text,
    payload.tooltip or "",
    payload.textColor or "on_surface"
  )
end

function update()
  noctalia.runAsync({ SCRIPT, "--bar" }, apply, TIMEOUT_MS)
end

function onClick()
  update()
end
