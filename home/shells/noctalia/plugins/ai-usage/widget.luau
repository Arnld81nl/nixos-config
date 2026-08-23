-- Bar widget showing how much of the Claude Code and Codex CLI subscription
-- limits are used.
--
-- All the work is in ~/.local/bin/ai-usage (home/home.nix); this only polls its
-- `--bar` JSON and paints it. That JSON is the same contract v4's CustomButton
-- parsed with parseJson:
--
--   {"text": "C 21%  X 10%", "icon": "", "tooltip": "...",
--    "textColor": "tertiary"|"error"}     -- colors only above 75% / 90%
--
-- Left click forces a re-read past the script's own response cache.

local SCRIPT = noctalia.expandPath("~/.local/bin/ai-usage")

-- ai-usage caches its HTTP responses for 120s, so polling faster than that only
-- re-reads the cache. A minute keeps the bar close to the CLIs' own numbers.
local POLL_MS = 60000

-- The two endpoints are remote; the default runAsync timeout is shorter than a
-- slow TLS handshake plus two requests.
local TIMEOUT_MS = 20000

local GLYPH = "sparkles"

noctalia.setUpdateInterval(POLL_MS)

-- Every field is set on every paint: a patch that omits one leaves the previous
-- value on screen, so an error state would otherwise keep the last good color.
local function render(text, tooltip, color)
  barWidget.setGlyph(GLYPH)
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
    render("ai -", "ai-usage timed out", "error")
    return
  end
  if result.exitCode ~= 0 then
    render("ai -", "ai-usage exited " .. tostring(result.exitCode) .. "\n" .. result.stderr, "error")
    return
  end

  local payload = noctalia.json.decode(result.stdout)
  if type(payload) ~= "table" or type(payload.text) ~= "string" then
    render("ai -", "ai-usage returned no usable JSON", "error")
    return
  end

  -- textColor is absent below 75% used; on_surface is the bar's own default.
  render(payload.text, payload.tooltip or "", payload.textColor or "on_surface")
end

local function poll(refresh)
  local argv = { SCRIPT, "--bar" }
  if refresh then
    table.insert(argv, "--refresh")
  end
  noctalia.runAsync(argv, apply, TIMEOUT_MS)
end

function update()
  poll(false)
end

function onClick()
  poll(true)
end
