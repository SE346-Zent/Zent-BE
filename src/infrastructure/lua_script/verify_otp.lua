local data = redis.call('GET', KEYS[1])
if not data then
    return -1
end

local json = cjson.decode(data)
local stored_code = json['code']
local attempts = tonumber(json['attempts'])

if stored_code == ARGV[1] then
    redis.call('DEL', KEYS[1])
    return 1
else
    attempts = attempts - 1
    if attempts <= 0 then
        redis.call('DEL', KEYS[1])
        return -3
    else
        json['attempts'] = attempts
        local ttl = redis.call('TTL', KEYS[1])
        if ttl > 0 then
            redis.call('SETEX', KEYS[1], ttl, cjson.encode(json))
        end
        return -2
    end
end
