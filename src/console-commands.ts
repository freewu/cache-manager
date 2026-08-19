/**
 * 命令行输入提示的命令字典
 * Redis 与 Memcached 命令区分：根据连接模式返回对应命令集合
 */

/** Redis 命令（官方全量，Redis 7.x，按字母序） */
export const REDIS_COMMANDS: string[] = [
  "APPEND", "AUTH", "BGREWRITEAOF", "BGSAVE", "BITCOUNT", "BITFIELD", "BITFIELD_RO",
  "BITOP", "BITPOS", "BLMPOP", "BLPOP", "BRPOP", "BRPOPLPUSH", "BZMPOP", "BZPOPMAX",
  "BZPOPMIN", "CLIENT", "CLUSTER", "COMMAND", "CONFIG", "COPY", "DBSIZE", "DEBUG",
  "DECR", "DECRBY", "DEL", "DISCARD", "DUMP", "ECHO", "EVAL", "EVALSHA", "EVALSHA_RO",
  "EVAL_RO", "EXEC", "EXISTS", "EXPIRE", "EXPIREAT", "EXPIRETIME", "FAILOVER",
  "FLUSHALL", "FLUSHDB", "FUNCTION", "GEOADD", "GEODIST", "GEOHASH", "GEOPOS",
  "GEORADIUS", "GEORADIUSBYMEMBER", "GEORADIUSBYMEMBER_RO", "GEORADIUS_RO",
  "GEOSEARCH", "GEOSEARCHSTORE", "GET", "GETBIT", "GETDEL", "GETEX", "GETRANGE",
  "GETSET", "HDEL", "HELLO", "HEXISTS", "HEXPIRE", "HEXPIREAT", "HEXPTTL", "HGET",
  "HGETALL", "HINCRBY", "HINCRBYFLOAT", "HKEYS", "HLEN", "HMGET", "HMSET",
  "HPERSIST", "HPEXPIRE", "HPEXPIREAT", "HPTTL", "HRANDFIELD", "HSCAN", "HSCAN_NOVALUES",
  "HSET", "HSETNX", "HSTRLEN", "HTTL", "HVALS", "INCR", "INCRBY", "INCRBYFLOAT",
  "INFO", "KEYS", "LASTSAVE", "LATENCY", "LCS", "LINDEX", "LINSERT", "LLEN",
  "LMOVE", "LMPOP", "LPOP", "LPOS", "LPUSH", "LPUSHX", "LRANGE", "LREM", "LSET",
  "LTRIM", "MEMORY", "MGET", "MIGRATE", "MODULE", "MONITOR", "MOVE", "MSET",
  "MSETNX", "MULTI", "OBJECT", "PERSIST", "PEXPIRE", "PEXPIREAT", "PEXPIRETIME",
  "PFADD", "PFCOUNT", "PFDEBUG", "PFMERGE", "PING", "PSETEX", "PSUBSCRIBE", "PSYNC",
  "PTTL", "PUBLISH", "PUBSUB", "PUNSUBSCRIBE", "QUIT", "RANDOMKEY", "READONLY",
  "READWRITE", "RENAME", "RENAMENX", "RESET", "RESTORE", "RESTORE-ASKING", "ROLE",
  "RPOP", "RPOPLPUSH", "RPUSH", "RPUSHX", "SADD", "SAVE", "SCAN", "SCARD", "SCRIPT",
  "SDIFF", "SDIFFSTORE", "SELECT", "SET", "SETBIT", "SETEX", "SETNX", "SETRANGE",
  "SHUTDOWN", "SINTER", "SINTERCARD", "SINTERSTORE", "SISMEMBER", "SLAVEOF",
  "SLOWLOG", "SMEMBERS", "SMISMEMBER", "SMOVE", "SORT", "SORT_RO", "SPOP", "SPUBLISH",
  "SRANDMEMBER", "SREM", "SSCAN", "STRLEN", "SUBSCRIBE", "SUBSTR", "SUNION",
  "SUNIONSTORE", "SWAPDB", "SYNC", "TIME", "TTL", "TYPE", "UNLINK", "UNSUBSCRIBE",
  "UNWATCH", "WAIT", "WATCH", "XACK", "XADD", "XAUTOCLAIM", "XCLAIM", "XDEL",
  "XGROUP", "XINFO", "XLEN", "XPENDING", "XRANGE", "XREAD", "XREADGROUP", "XREVRANGE",
  "XSETID", "XTRIM", "ZADD", "ZCARD", "ZCOUNT", "ZDIFF", "ZDIFFSTORE", "ZINCRBY",
  "ZINTER", "ZINTERCARD", "ZINTERSTORE", "ZLEXCOUNT", "ZMPOP", "ZMSCORE", "ZPOPMAX",
  "ZPOPMIN", "ZRANDMEMBER", "ZRANGE", "ZRANGEBYLEX", "ZRANGEBYSCORE", "ZRANGESTORE",
  "ZRANK", "ZREM", "ZREMRANGEBYLEX", "ZREMRANGEBYRANK", "ZREMRANGEBYSCORE",
  "ZREVRANGE", "ZREVRANGEBYLEX", "ZREVRANGEBYSCORE", "ZREVRANK", "ZSCAN", "ZSCORE",
  "ZUNION", "ZUNIONSTORE",
];

/** Memcached 命令（小写，与协议一致） */
export const MEMCACHED_COMMANDS: string[] = [
  "get", "gets", "gat", "gats",
  "set", "add", "replace", "append", "prepend", "cas",
  "delete", "incr", "decr", "touch",
  "stats", "stats items", "stats slabs", "stats cachedump", "stats sizes",
  "stats settings", "stats conns", "stats memory", "stats detail",
  "flush_all", "version", "verbosity", "quit",
  "lru_crawler", "lru_crawler metadump", "cache_memlimit",
  "sasl_list", "sasl_auth", "watch",
];

/** 根据连接模式返回命令集合 */
export function commandsForMode(mode?: string): string[] {
  return mode === "memcached" ? MEMCACHED_COMMANDS : REDIS_COMMANDS;
}

/** 匹配命令提示（前缀匹配，忽略大小写，最多 limit 条） */
export function matchCommands(mode: string | undefined, prefix: string, limit = 8): string[] {
  const list = commandsForMode(mode);
  const p = prefix.trim().toLowerCase();
  if (!p) return [];
  return list.filter((c) => c.toLowerCase().startsWith(p)).slice(0, limit);
}
