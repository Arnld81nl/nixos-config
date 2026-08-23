-- One VPN's bar button: shows whether that VPN is connected, and toggles it.
--
-- Three instances of this entry sit in the `vpn` accordion capsule (see
-- config.toml); `vpn` is a per-widget setting holding "1", "2" or "3".
--
-- State comes from `vpn-status --bar N` (home/home.nix):
--
--   {"text": "<name>", "glyph": "shield"|"shield-check",
--    "tooltip": "<name> — connected, click to disconnect",
--    "textColor": "primary"|"on_surface"}
--
-- Clicking runs ~/.local/bin/vpn-<N>. Those scripts report what they are doing
-- on stdout, which goes nowhere when the shell spawns them — so their output is
-- turned into a desktop notification here. Without that a click looks like it
-- did nothing for the 10-30s a connection takes.

local INDEX = tostring(noctalia.getConfig("vpn") or "1")

local STATUS = noctalia.expandPath("~/.local/bin/vpn-status")
local TOGGLE = noctalia.expandPath("~/.local/bin/vpn-" .. INDEX)

-- The checks are local (pgrep plus `sudo -n swanctl --list-sas`), so polling is
-- cheap; 5s keeps the button in step with a toggle without visible lag.
local POLL_MS = 5000
local STATUS_TIMEOUT_MS = 5000

-- Connecting reads credentials from 1Password (which may raise its own approval
-- prompt) and waits on the tunnel, so this has to be far longer than a poll.
local TOGGLE_TIMEOUT_MS = 180000

-- Best-effort label until the first poll answers; replaced with the real name.
local name = "VPN " .. INDEX

-- While a toggle is in flight the button shows "working" and polling is paused,
-- so the 5s tick cannot repaint over it.
local busy = false

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

-- The scripts print progress line by line; the last non-empty line is the
-- outcome ("Connected to X VPN.", "Disconnected.", or the error).
local function lastLine(text)
  local result = ""
  for line in string.gmatch(text or "", "[^\r\n]+") do
    if string.match(line, "%S") then
      result = line
    end
  end
  return result
end

local function applyStatus(result)
  if result.timedOut then
    render("shield-x", name, "vpn-status timed out", "error")
    return
  end
  if result.exitCode ~= 0 then
    render("shield-x", name, "vpn-status exited " .. tostring(result.exitCode), "error")
    return
  end

  local payload = noctalia.json.decode(result.stdout)
  if type(payload) ~= "table" or type(payload.text) ~= "string" then
    render("shield-x", name, "vpn-status returned no usable JSON", "error")
    return
  end

  name = payload.text
  render(payload.glyph or "shield", name, payload.tooltip or "", payload.textColor or "on_surface")
end

function update()
  if busy then
    return
  end
  noctalia.runAsync({ STATUS, "--bar", INDEX }, applyStatus, STATUS_TIMEOUT_MS)
end

function onClick()
  if busy then
    return
  end
  busy = true
  render("shield-bolt", name, name .. " — working…", "tertiary")

  noctalia.runAsync({ TOGGLE }, function(result)
    busy = false

    if result.timedOut then
      noctalia.notifyError(name, "Timed out")
    elseif result.exitCode ~= 0 then
      local message = lastLine(result.stderr)
      if message == "" then
        message = lastLine(result.stdout)
      end
      noctalia.notifyError(name, if message ~= "" then message else "Failed (exit " .. tostring(result.exitCode) .. ")")
    else
      local message = lastLine(result.stdout)
      noctalia.notify(name, if message ~= "" then message else "Done")
    end

    update()
  end, TOGGLE_TIMEOUT_MS)
end
