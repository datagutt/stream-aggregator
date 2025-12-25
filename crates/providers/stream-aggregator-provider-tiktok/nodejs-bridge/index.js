#!/usr/bin/env node

const { TikTokLiveConnection } = require('tiktok-live-connector');
const readline = require('readline');

class TikTokBridge {
    constructor() {
        this.connections = new Map();
    }

    async getRoomInfo(username) {
        try {
            const tiktokConnection = new TikTokLiveConnection(username);
            
            let roomInfo = null;
            try {
                roomInfo = await tiktokConnection.fetchRoomInfo();
            } catch (err) {
                return {
                    success: false,
                    error: `Failed to get room info: ${err.message}`
                };
            }

            if (!roomInfo) {
                return {
                    success: true,
                    data: {
                        name: username,
                        avatar: '',
                        live: false,
                        title: null,
                        viewers: null,
                        thumbnail_url: null
                    }
                };
            }

            return {
                success: true,
                data: {
                    live: roomInfo.status === 2,
                    name: roomInfo.owner?.display_id || username,
                    avatar: roomInfo.owner?.avatar_large?.url_list?.[1] || '',
                    thumbnail_url: roomInfo.cover?.url_list?.[1] || null,
                    viewers: roomInfo.user_count || null,
                    title: roomInfo.title || null
                }
            };
        } catch (error) {
            return {
                success: false,
                error: error.message
            };
        }
    }

    async handleCommand(command) {
        try {
            const { action, username } = command;

            switch (action) {
                case 'get_room_info':
                    return await this.getRoomInfo(username);
                
                case 'ping':
                    return { success: true, pong: true };
                
                default:
                    return { 
                        success: false, 
                        error: `Unknown action: ${action}` 
                    };
            }
        } catch (error) {
            return {
                success: false,
                error: error.message
            };
        }
    }

    start() {
        const rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout,
            terminal: false
        });

        console.error('TikTok Bridge Started');

        rl.on('line', async (line) => {
            try {
                const command = JSON.parse(line);
                const response = await this.handleCommand(command);
                console.log(JSON.stringify(response));
            } catch (error) {
                console.log(JSON.stringify({
                    success: false,
                    error: `Parse error: ${error.message}`
                }));
            }
        });

        rl.on('close', () => {
            console.error('TikTok Bridge Closed');
            process.exit(0);
        });
    }
}

const bridge = new TikTokBridge();
bridge.start();
