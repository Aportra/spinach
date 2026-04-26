import ollama
# import os
import discord
import asyncio
import yaml
import draw
import bot_commands
from psql import psql
# from spinach import look
from spinach import req_news
from spinach import search
from datetime import datetime, timedelta
from pathlib import Path

home = Path.home()
try:
    with open(f'{home}/spinach/rag-parsing/config.yaml', 'r') as f:
        data = yaml.load(f, Loader=yaml.SafeLoader)
except FileNotFoundError:
    with open(f'{home}/spinach/rag-parsing/config-default.yaml', 'r') as f:
        data = yaml.load(f, Loader=yaml.SafeLoader)
model = data.get("model")
discord_key = data.get("discord_key")
intents = discord.Intents.default()
intents.message_content = True
client = discord.Client(intents=intents)

messages = [
    {
        "role": "system",
        "content": (
            "You're a helpful assistant"
            "Make sure the message will fit in discord rules of 4000 words or less"
            f"Current date is {datetime.now()}"
        ),
    }
]
max_history = 20
schema_messages = []



def split_message(text, limit=2000):
    chunks = []
    while len(text) > limit:
        split_at = text.rfind('\n', 0, limit)
        if split_at == -1:
            split_at = text.rfind(' ', 0, limit)
            if split_at == -1:
                split_at = limit  # no whitespace found, hard cut
        chunks.append(text[:split_at])
        text = text[split_at:].lstrip()
    chunks.append(text)
    return chunks


@client.event
async def on_ready():
    print(f"Logged in as {client.user} and ready")


@client.event
async def on_message(message):
    commands = ({"!set_schema":lambda:bot_commands.set_schema(message.content.split(' ', 1)[1],schema_messages),
                 "!query":lambda:bot_commands.query(message.content.split(' ', 1)[1], schema_messages),
                 "!search":lambda:bot_commands.search_fn(message.content.split(' ', 1)[1], messages),
                 "!news":lambda:bot_commands.news_fn(messages)})
    if message.channel.id != 1496956603612532766:
        return
    if message.author == client.user:
        return
    if message.content.startswith("!"):
        com = message.content.split(' ', 1)[0]
        if com not in commands.keys():
            await message.channel.send(f"not valid command:{com}")
            return
        buf, response = await commands[com]()
        if response is not None:
            chunks = split_message(response)
            for chunk in chunks:
                await message.channel.send(chunk)   # messages.append
            # await message.channel.send(response)

        if buf is not None:
            await message.channel.send(file=discord.File(buf, filename="table.png"))
            buf.close()

        return

    messages.append({"role": "user", "content": message.content})
    print(f"Message received: {message.content}")

    response = await asyncio.get_event_loop().run_in_executor(None,lambda:ollama.chat(model=model,
                            messages=messages,
                            stream=False))
    content = response["message"]["content"]
    messages.append({"role": "assistant", "content": content})
    if len(messages) > max_history:
        messages[1:] = messages[-(max_history-1):]
    chunks = split_message(content)

    for chunk in chunks:
        await message.channel.send(chunk)   # messages.append

client.run(discord_key)
