import ollama
# import os
import discord
import asyncio
import yaml
import draw
from psql import psql
# from spinach import look
from spinach import req_news
from spinach import search
from datetime import datetime, timedelta
from pathlib import Path


model = 'gemma4:e4b'


async def set_schema(schema_set, schema_messages):

    if len(schema_messages) > 0:
        schema_messages.clear()
    conn = psql()
    q = f"""
    SELECT
        column_name,
        data_type,
        is_nullable
    FROM
        information_schema.columns
    WHERE
        table_name = '{schema_set}'
        AND table_schema = 'public'; -- Specify schema if necessary
                        """
    results = await asyncio.get_event_loop().run_in_executor(None, conn.query, q)

    schema = results.to_markdown()

    schema_messages.append((schema_set, schema))
    conn.close()
    return None, None


async def query(msg, schema_messages):

    conn = psql()
    if len(schema_messages) > 0:
        response = await asyncio.get_event_loop().run_in_executor(
                None,lambda:(ollama.chat(model=model,
                                messages=([{"role": "system", "content": f"only provide a sql query for postgresql nothing else.ensure this query is in markdown code block. Table to use:{schema_messages[0][0]}  Schema of the table:{schema_messages[0][1]}"},
                                            {"role": "user", "content": msg}]),
                                stream=False)))
    else:
        response = await asyncio.get_event_loop().run_in_executor(
                None,lambda:(ollama.chat(model=model,
                                messages=([{"role": "system", "content": "only provide a sql query for postgresql nothing else.ensure this query is in markdown code block."},
                                            {"role": "user", "content": msg}]),
                                stream=False)))
    query = response["message"]["content"].replace('`','').replace('sql','')
    # messages.append({"role": "user", "content": message.content.split(' ',1)[1]})
    # messages.append({"role": "system", "content":response["message"]["content"].replace('`','').replace('sql','')})

    results = conn.query(query)

    buf = await asyncio.get_event_loop().run_in_executor(None,
                                            draw.table, results)

    return buf, response['message']['content']


async def search_fn(sm, messages):
    search_result = await asyncio.get_event_loop().run_in_executor(
            None, search, sm)
    search_message = ''
    for key in search_result:
        search_message += key + ' '
        for i in range(len(search_result[key])):
            search_message += search_result[key][i]
    response = (ollama.chat(model=model,
                            messages=([{"role": "system", "content": "summarize the following search results and include clickable links."},
                                        {"role": "user", "content": search_message}]),
                            stream=False))
    messages.append({"role": "user", "content": search_message})

    return None, response['message']['content']


async def news_fn(messages):
    news = await asyncio.get_event_loop().run_in_executor(
            None, lambda: req_news(q=None, search=None, num=10))
    news_message = 'here is the news please summarize and include the links and ensure they are clickable'
    for key in news:
        news_message += key + ' '
        for i in range(len(news[key])):
            news_message += news[key][i]

    response = await asyncio.get_event_loop().run_in_executor(
            None, lambda: (ollama.chat(model=model,
                messages=([{"role": "system", "content": "Summarize the following news headlines and include clickable links alongside the headlines. group by theme"},
                            {"role": "user", "content": news_message}]),
                            stream=False)))

    return None, response['message']['content']

async def lookup(message, messages):
    print(message)

# async def save_chat(messages):


