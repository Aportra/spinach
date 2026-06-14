import ollama
import discord
import asyncio
import yaml
import bot_commands
import pandas as pd
import base64
from pathlib import Path
from datetime import datetime as dt
from spinach import look
from psql import psql


home = Path.home()
try:
    with open(f'{home}/spinach/rag-parsing/config.yaml', 'r') as f:
        data = yaml.load(f, Loader=yaml.SafeLoader)
except FileNotFoundError:
    with open(f'{home}/spinach/rag-parsing/config-default.yaml', 'r') as f:
        data = yaml.load(f, Loader=yaml.SafeLoader)
model = data.get("model")
thinking_model = data.get("thinking_model")
discord_key = data.get("discord_key")
parent_id = data.get("parent_id")
intents = discord.Intents.default()
intents.message_content = True
client = discord.Client(intents=intents)
last_message = None
messages = [
    {
        "role": "system",
        "content": (
            "You're a helpful assistant"
            f"Current date is {dt.now()}"
        ),
    }
]
max_history = 20
schema_messages = []

threads = {"chat_id": [], "summary": [], "date_of_chat": []}

active_thread = []


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
    forum = client.get_channel(parent_id)
    print(parent_id)
    if forum:
        for thread in forum.threads:
            if thread.id not in threads["chat_id"]:
                threads["chat_id"].append(thread.id)

    print(len(threads["chat_id"]))


@client.event
async def on_message(message):

    db = psql()

    if message.author == client.user:
        return
    commands = ({"!set_schema": lambda: (bot_commands
                                         .set_schema(
                                            message.content.split(' ', 1)[1],
                                            schema_messages)),
                 "!query": lambda: (bot_commands
                                    .query(message.content.split(' ', 1)[1],
                                           schema_messages)),
                 "!search": lambda: (bot_commands
                                     .search_fn(message.content.split(' ', 1)[1],
                                                messages)),
                 "!news": lambda: bot_commands.news_fn(),
                "think!": lambda: (bot_commands.think(message.content.split(' ', 1)[1],
                                                    messages))
                 })

    global last_message

    lm = dt.strptime(str(message.created_at).split('.')[0], "%Y-%m-%d %H:%M:%S")
    last_message = lm
    if message.channel.id not in threads["chat_id"]:
        return
    if message.author == client.user:
        return
    if message.channel.id not in active_thread:
        if message.channel.id not in threads["chat_id"]:
            messages.clear()
            active_thread.clear()
            threads["chat_id"].append(message.channel_id)
            active_thread.append(message.channel.id)
        else:
            messages.clear()
            active_thread.clear()
            active_thread.append(message.channel.id)
    if message.content.startswith("!"):
        com = message.content.split(' ', 1)[0]
        if com not in commands.keys():
            await message.channel.send(f"not valid command:{com}")
            return
        buf, response = await commands[com]()
        if response is not None:
            # data_upload = ({"chat_id": message.channel.id,
            #                 "user_msg": message.content,
            #                 "bot_msg": response,
            #                 "created_at": last_message})
            # db.upload_data(pd.DataFrame(data_upload, index=[0]), "chat_log")
            # db.close()
            chunks = split_message(response)
            for chunk in chunks:
                await message.channel.send(chunk)   # messages.append
            # await message.channel.send(response)

        if buf is not None:
            await message.channel.send(file=discord.File(buf, filename="table.png"))
            buf.close()

        return
    if message.attachments:
        print("Reached")
        files = {}
        images = []
        image_ext = ('.png', '.jpg', '.jpeg', '.webp', '.gif')
        for attachment in message.attachments:
            file_content = await attachment.read()
            if attachment.content_type.startswith("image/") or attachment.filename.lower().endswith(image_ext):
                images.append(base64.b64encode(file_content).decode('utf-8'))
            else:
                files[f'{attachment.filename}'] = file_content.decode('utf-8')

        if len(images) > 0:
            m = {"role": "user", "content": message.content, "images": images}
        else:
            print("Reached")
            data = await asyncio.get_event_loop().run_in_executor(
                    None, lambda: look(context= message.content, file= files))
            m = {"role": "user", "content": "here is the file:"+str(data)+message.content}

        response = await asyncio.get_event_loop().run_in_executor(None,lambda:ollama.chat(model=model,
                                messages=[m],
                                stream=False))
        chunks = split_message(response["message"]["content"])
        for chunk in chunks:
            await message.channel.send(chunk)   # messages.append
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

    data_upload = ({"chat_id": message.id,
                    "user_msg": message.content,
                    "bot_msg": content,
                    "created_at": last_message})

    print(data_upload)
    for chunk in chunks:
        await message.channel.send(chunk)   # messages.append
    # db.upload_data(pd.DataFrame(data_upload, index=[0]), "chat_log")

    # db.close()


@client.event
async def on_thread_create(thread):
    if thread.parent_id == parent_id:
        starter = await thread.fetch_message(thread.id)
        threads["chat_id"].append(thread.id)
        await on_message(starter)


@client.event
async def on_thread_delete(thread):
    messages.clear()


client.run(discord_key)
