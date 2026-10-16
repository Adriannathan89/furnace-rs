-- Atomic room lifecycle, membership, and history visibility on one Redis endpoint.
local meta,members,stream,lease,channel=unpack(KEYS)
local op,instance=ARGV[1],ARGV[2]
if redis.call('GET',lease)~=instance then return {'error','backend_unavailable'} end
local t=redis.call('TIME');local now=tonumber(t[1])*1000+math.floor(tonumber(t[2])/1000)
local function err(code) return {'error',code} end
local function less(a,b)
 local am,as=string.match(a,'^(%d+)%-(%d+)$');local bm,bs=string.match(b,'^(%d+)%-(%d+)$')
 return tonumber(am)<tonumber(bm) or (tonumber(am)==tonumber(bm) and tonumber(as)<tonumber(bs))
end
if op=='create' then
 if redis.call('HGET',meta,'definition') and redis.call('HGET',meta,'deleted')~='1' then
  if redis.call('HGET',meta,'definition')~=ARGV[3] then return err('room_conflict') end
  return 'ok'
 end
 local previous=redis.call('HGET',meta,'high') or '0-0';local pm,ps=string.match(previous,'^(%d+)%-(%d+)$')
 local id=tostring(math.max(now,tonumber(pm)))..'-'..tostring(now<=tonumber(pm) and tonumber(ps)+1 or 0)
 redis.call('XADD',stream,id,'boundary','1');redis.call('XDEL',stream,id)
 redis.call('HSET',meta,'definition',ARGV[3],'generation',ARGV[4],'age',ARGV[5],'count',ARGV[6],'replay',ARGV[7],'created',id,'high',id,'trimmed','0-0','deleted','0')
 return 'ok'
end
if op=='broadcast' then redis.call('PUBLISH',channel,ARGV[3]);return 'ok' end
if redis.call('HGET',meta,'deleted')~='0' then return err('room_not_found') end
redis.call('ZREMRANGEBYSCORE',members,'-inf',now)
local function trim()
 if redis.call('HGET',meta,'replay')~='1' then return end
 local age=tonumber(redis.call('HGET',meta,'age'));local count=tonumber(redis.call('HGET',meta,'count'))
 local cutoff=math.max(0,now-age)
 local expired=redis.call('XREVRANGE',stream,tostring(cutoff)..'-18446744073709551615','-','COUNT',1)
 local removed=redis.call('HGET',meta,'trimmed')
 if #expired>0 and less(removed,expired[1][1]) then removed=expired[1][1] end
 redis.call('XTRIM',stream,'MINID','=',tostring(cutoff+1)..'-0')
 local excess=redis.call('XLEN',stream)-count
 if excess>0 then local entries=redis.call('XRANGE',stream,'-','+','COUNT',excess);for _,v in ipairs(entries) do redis.call('XDEL',stream,v[1]);if less(removed,v[1]) then removed=v[1] end end end
 redis.call('HSET',meta,'trimmed',removed)
end
trim()
if op=='trim' then return 'ok' end
if op=='info' then return {redis.call('HGET',meta,'definition'),redis.call('HGET',meta,'generation'),redis.call('HGET',meta,'high')} end
if op=='delete' then
 if redis.call('ZCARD',members)>0 then return err('room_not_empty') end
 redis.call('HSET',meta,'deleted','1');redis.call('DEL',members,stream);return 'ok'
end
if op=='join' or op=='renew_member' then
 if op=='renew_member' and redis.call('HGET',meta,'generation')~=ARGV[4] then return err('history_expired') end
 if op=='join' and ARGV[4]~='' then
  if redis.call('HGET',meta,'replay')~='1' then return err('invalid_cursor') end
  if less(ARGV[4],redis.call('HGET',meta,'created')) or less(ARGV[4],redis.call('HGET',meta,'trimmed')) then return err('history_expired') end
  if less(redis.call('HGET',meta,'high'),ARGV[4]) then return err('invalid_cursor') end
 end
 redis.call('ZADD',members,now+redis.call('PTTL',lease),instance..':'..ARGV[3])
 return {redis.call('HGET',meta,'definition'),redis.call('HGET',meta,'generation'),redis.call('HGET',meta,'high')}
end
if op=='leave' then redis.call('ZREM',members,instance..':'..ARGV[3]);return 'ok' end
if op=='publish' then
 if redis.call('HGET',meta,'replay')=='1' then return err('invalid_message') end
 local packet=cjson.decode(ARGV[3]);packet.generation=redis.call('HGET',meta,'generation');redis.call('PUBLISH',channel,cjson.encode(packet));return 'ok'
end
if op=='append' then
 if redis.call('HGET',meta,'replay')~='1' then return err('invalid_message') end
 local id=redis.call('XADD',stream,'*','event',ARGV[3]);redis.call('HSET',meta,'high',id);trim();return id
end
if op=='read' then
 if redis.call('HGET',meta,'generation')~=ARGV[3] then return err('history_expired') end
 if less(ARGV[4],redis.call('HGET',meta,'trimmed')) then return err('history_expired') end
 return redis.call('XRANGE',stream,'('..ARGV[4],ARGV[5],'COUNT',64)
end
return err('invalid_message')
