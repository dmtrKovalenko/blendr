-- Example external decoder, loaded via `script` in config_example.toml.
--
-- Available globals: data, bytes, len, hex, and read(fmt, offset).
-- Return a string (or number) to display in the connection view.
--
-- This one decodes a small status packet:
--   byte 0    = flags
--   bytes 1-2 = little-endian battery millivolts

if len == 0 then
  return "(empty)"
end

local flags = bytes[1]
local battery_mv = read("<I2", 1)

return string.format("flags=0x%02x  battery=%dmV  (%d bytes: %s)", flags, battery_mv, len, hex)
