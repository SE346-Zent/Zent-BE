local key = KEYS[1]
local pending_val = ARGV[1]
local ttl = ARGV[2]

local current = redis.call('GET', key)
if current then
    return current
else
    redis.call('SET', key, pending_val, 'EX', ttl)
    return nil
end
