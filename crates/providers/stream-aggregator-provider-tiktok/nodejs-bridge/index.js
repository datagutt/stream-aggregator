#!/usr/bin/env node

const http = require('node:http');
const { TikTokLiveConnection } = require('tiktok-live-connector');

const CONFIG = {
	port: parseInt(process.env.TIKTOK_BRIDGE_PORT || '3456', 10),
	maxConcurrentRequests: parseInt(process.env.TIKTOK_MAX_CONCURRENT || '10', 10),
	requestTimeoutMs: parseInt(process.env.TIKTOK_REQUEST_TIMEOUT || '30000', 10),
	connectionCacheTtlMs: parseInt(process.env.TIKTOK_CONNECTION_TTL || '300000', 10),
	cleanupIntervalMs: 30000,
	responseCacheTtlMs: parseInt(process.env.TIKTOK_RESPONSE_CACHE_TTL || '30000', 10),
};

const ErrorCode = {
	UNKNOWN: 'unknown_error',
	USER_NOT_FOUND: 'user_not_found',
	USER_OFFLINE: 'user_offline',
	RATE_LIMITED: 'rate_limited',
	INVALID_RESPONSE: 'invalid_response',
	TIMEOUT: 'timeout',
	NETWORK_ERROR: 'network_error',
	CAPTCHA: 'captcha_required',
};

function parseError(err) {
	const message = err?.message || err?.toString() || 'Unknown error';
	const name = err?.name || err?.constructor?.name || 'Error';

	if (message.includes('LIVE has ended') || message.includes('not found') || message.includes('isn\'t online')) {
		return { code: ErrorCode.USER_OFFLINE, message };
	}

	if (name === 'UserOfflineError') {
		return { code: ErrorCode.USER_OFFLINE, message };
	}

	if (name === 'InvalidUniqueIdError' || message.includes('Invalid uniqueId')) {
		return { code: ErrorCode.USER_NOT_FOUND, message };
	}

	if (name === 'SignatureRateLimitError' || message.includes('Rate Limit') || message.includes('rate limit')) {
		const retryAfter = err?.retryAfter || null;
		return { code: ErrorCode.RATE_LIMITED, message, retryAfter };
	}

	if (name === 'InvalidResponseError' || message.includes('Invalid response')) {
		return { code: ErrorCode.INVALID_RESPONSE, message };
	}

	if (message.includes('timeout') || message.includes('Timeout')) {
		return { code: ErrorCode.TIMEOUT, message };
	}

	if (message.includes('ECONNREFUSED') || message.includes('ENOTFOUND') || message.includes('network')) {
		return { code: ErrorCode.NETWORK_ERROR, message };
	}

	if (message.includes('captcha') || message.includes('Captcha')) {
		return { code: ErrorCode.CAPTCHA, message };
	}

	return { code: ErrorCode.UNKNOWN, message };
}

class ResponseCache {
	constructor(ttlMs) {
		this.cache = new Map();
		this.ttlMs = ttlMs;
	}

	get(key) {
		const entry = this.cache.get(key);
		if (!entry) return null;

		if (Date.now() - entry.timestamp > this.ttlMs) {
			this.cache.delete(key);
			return null;
		}

		return entry.data;
	}

	set(key, data) {
		this.cache.set(key, {
			data,
			timestamp: Date.now(),
		});
	}

	cleanup() {
		const now = Date.now();
		for (const [key, entry] of this.cache.entries()) {
			if (now - entry.timestamp > this.ttlMs) {
				this.cache.delete(key);
			}
		}
	}

	clear() {
		this.cache.clear();
	}

	size() {
		return this.cache.size;
	}
}

class RequestQueue {
	constructor(maxConcurrent) {
		this.maxConcurrent = maxConcurrent;
		this.running = 0;
		this.queue = [];
	}

	async add(fn) {
		return new Promise((resolve, reject) => {
			const execute = async () => {
				this.running++;
				try {
					const result = await fn();
					resolve(result);
				} catch (error) {
					reject(error);
				} finally {
					this.running--;
					this.processNext();
				}
			};

			if (this.running < this.maxConcurrent) {
				execute();
			} else {
				this.queue.push(execute);
			}
		});
	}

	processNext() {
		if (this.queue.length > 0 && this.running < this.maxConcurrent) {
			const next = this.queue.shift();
			next();
		}
	}

	stats() {
		return {
			running: this.running,
			queued: this.queue.length,
		};
	}
}

class TikTokBridge {
	constructor() {
		this.responseCache = new ResponseCache(CONFIG.responseCacheTtlMs);
		this.requestQueue = new RequestQueue(CONFIG.maxConcurrentRequests);
		this.stats = {
			totalRequests: 0,
			successfulRequests: 0,
			failedRequests: 0,
			cacheHits: 0,
		};
		this.startTime = Date.now();

		this.cleanupInterval = setInterval(() => {
			this.responseCache.cleanup();
		}, CONFIG.cleanupIntervalMs);
	}

	async createConnection(username, options = {}) {
		const connectionOptions = {
			processInitialData: false,
			fetchRoomInfoOnConnect: false,
			enableExtendedGiftInfo: false,
			requestPollingIntervalMs: 1000,
			...options,
		};

		return new TikTokLiveConnection(username, connectionOptions);
	}

	async getRoomInfo(username, options = {}) {
		this.stats.totalRequests++;

		const cached = this.responseCache.get(username);
		if (cached) {
			this.stats.cacheHits++;
			return cached;
		}

		return this.requestQueue.add(async () => {
			const timeoutPromise = new Promise((_, reject) => {
				setTimeout(() => reject(new Error('Request timeout')), CONFIG.requestTimeoutMs);
			});

			const fetchPromise = this.fetchRoomInfoInternal(username, options);

			try {
				const result = await Promise.race([fetchPromise, timeoutPromise]);
				this.stats.successfulRequests++;
				if (result.success) {
					this.responseCache.set(username, result);
				}
				return result;
			} catch (error) {
				this.stats.failedRequests++;
				throw error;
			}
		});
	}

	async fetchRoomInfoInternal(username, options = {}) {
		try {
			const connection = await this.createConnection(username, options);

			let roomInfo = null;
			let isLive = false;

			try {
				roomInfo = await connection.fetchRoomInfo();
			} catch (err) {
				const parsedErr = parseError(err);

				if (parsedErr.code === ErrorCode.USER_OFFLINE || parsedErr.code === ErrorCode.USER_NOT_FOUND) {
					return {
						success: true,
						data: {
							username: username,
							display_name: username,
							avatar_url: null,
							live: false,
							title: null,
							viewer_count: null,
							thumbnail_url: null,
							stream_url: null,
							room_id: null,
							create_time: null,
							bio: null,
						},
					};
				}

				return {
					success: false,
					error: parsedErr.message,
					error_code: parsedErr.code,
					retry_after: parsedErr.retryAfter || null,
				};
			}

			if (!roomInfo || !roomInfo.data) {
				return {
					success: true,
					data: {
						username: username,
						display_name: username,
						avatar_url: null,
						live: false,
						title: null,
						viewer_count: null,
						thumbnail_url: null,
						stream_url: null,
						room_id: null,
						create_time: null,
						bio: null,
					},
				};
			}

			const data = roomInfo.data;
			isLive = data.status === 2;

			return {
				success: true,
				data: {
					live: isLive,
					username: username,
					display_name: data.owner?.display_id || data.owner?.nickname || username,
					avatar_url: data.owner?.avatar_large?.url_list?.[0] ||
						data.owner?.avatar_medium?.url_list?.[0] ||
						data.owner?.avatar_thumb?.url_list?.[0] || null,
					thumbnail_url: data.cover?.url_list?.[0] || null,
					viewer_count: data.user_count || data.like_count || null,
					title: data.title || null,
					stream_url: data.stream_url?.hls_pull_url || null,
					room_id: data.id_str || String(data.id) || null,
					create_time: data.create_time || null,
					bio: data.owner?.bio_description || null,
				},
			};
		} catch (error) {
			const parsedErr = parseError(error);
			return {
				success: false,
				error: parsedErr.message,
				error_code: parsedErr.code,
				retry_after: parsedErr.retryAfter || null,
			};
		}
	}

	async getBatchRoomInfo(usernames, options = {}) {
		const startTime = Date.now();
		const results = await Promise.allSettled(
			usernames.map(async (username) => {
				try {
					const result = await this.getRoomInfo(username, options);
					return { username, ...result };
				} catch (error) {
					const parsedErr = parseError(error);
					return {
						username,
						success: false,
						error: parsedErr.message,
						error_code: parsedErr.code,
					};
				}
			})
		);

		const processedResults = results.map((result, index) => {
			if (result.status === 'fulfilled') {
				return result.value;
			}
			const parsedErr = parseError(result.reason);
			return {
				username: usernames[index],
				success: false,
				error: parsedErr.message,
				error_code: parsedErr.code,
			};
		});

		const successful = processedResults.filter(r => r.success).length;
		const failed = processedResults.length - successful;

		return {
			success: true,
			results: processedResults,
			stats: {
				total: usernames.length,
				successful,
				failed,
				duration_ms: Date.now() - startTime,
			},
		};
	}

	getHealth() {
		return {
			status: 'ok',
			uptime: Date.now() - this.startTime,
			activeConnections: this.requestQueue.stats().running,
			totalRequests: this.stats.totalRequests,
			cacheSize: this.responseCache.size(),
		};
	}

	getStats() {
		return {
			success: true,
			stats: {
				total_requests: this.stats.totalRequests,
				successful_requests: this.stats.successfulRequests,
				failed_requests: this.stats.failedRequests,
				cache_hits: this.stats.cacheHits,
				cache_size: this.responseCache.size(),
				queue_stats: this.requestQueue.stats(),
				uptime_ms: Date.now() - this.startTime,
			},
		};
	}

	parseBody(req) {
		return new Promise((resolve, reject) => {
			let body = '';
			req.on('data', chunk => {
				body += chunk.toString();
			});
			req.on('end', () => {
				if (!body) {
					resolve({});
					return;
				}
				try {
					resolve(JSON.parse(body));
				} catch (err) {
					reject(new Error('Invalid JSON: ' + err.message));
				}
			});
			req.on('error', reject);
		});
	}

	sendJson(res, statusCode, data) {
		res.writeHead(statusCode, { 'Content-Type': 'application/json' });
		res.end(JSON.stringify(data));
	}

	async handleRequest(req, res) {
		const url = new URL(req.url, `http://localhost:${CONFIG.port}`);
		const path = url.pathname;

		res.setHeader('Access-Control-Allow-Origin', '*');
		res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
		res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

		if (req.method === 'OPTIONS') {
			res.writeHead(204);
			res.end();
			return;
		}

		try {
			if (path === '/health' && req.method === 'GET') {
				this.sendJson(res, 200, this.getHealth());
				return;
			}

			if (path === '/stats' && req.method === 'GET') {
				this.sendJson(res, 200, this.getStats());
				return;
			}

			if (path === '/room' && req.method === 'POST') {
				const body = await this.parseBody(req);
				if (!body.username) {
					this.sendJson(res, 400, {
						success: false,
						error: 'username is required',
						error_code: 'invalid_request'
					});
					return;
				}
				const result = await this.getRoomInfo(body.username, body.options || {});
				this.sendJson(res, result.success ? 200 : 500, result);
				return;
			}

			if (path === '/batch' && req.method === 'POST') {
				const body = await this.parseBody(req);
				if (!Array.isArray(body.usernames)) {
					this.sendJson(res, 400, {
						success: false,
						error: 'usernames must be an array',
						error_code: 'invalid_request'
					});
					return;
				}
				const result = await this.getBatchRoomInfo(body.usernames, body.options || {});
				this.sendJson(res, 200, result);
				return;
			}

			this.sendJson(res, 404, {
				success: false,
				error: 'Not found',
				error_code: 'not_found'
			});
		} catch (error) {
			console.error('Request error:', error);
			const parsedErr = parseError(error);
			this.sendJson(res, 500, {
				success: false,
				error: parsedErr.message,
				error_code: parsedErr.code
			});
		}
	}

	start() {
		const server = http.createServer((req, res) => this.handleRequest(req, res));

		server.listen(CONFIG.port, '127.0.0.1', () => {
			console.log(`TikTok Bridge HTTP Server started on http://127.0.0.1:${CONFIG.port}`);
			console.log('Endpoints:');
			console.log(`  GET  /health - Health check`);
			console.log(`  GET  /stats  - Bridge statistics`);
			console.log(`  POST /room   - Get room info for a single user`);
			console.log(`  POST /batch  - Get room info for multiple users`);
		});

		const shutdown = () => {
			console.log('\nShutting down...');
			clearInterval(this.cleanupInterval);
			server.close(() => {
				console.log('Server closed');
				process.exit(0);
			});
		};

		process.on('SIGTERM', shutdown);
		process.on('SIGINT', shutdown);
	}
}

const bridge = new TikTokBridge();
bridge.start();
