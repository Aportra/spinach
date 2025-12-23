#!/home/aportra99/spinach-rag/ai/bin/python
import ollama
import yaml
import re
import sys
from spinach import look
from spinach import req_news
from spinach import search
from pathlib import Path
from datetime import datetime, timedelta

us_sources = [
"abc-news",
"associated-press",
"axios",
"breitbart-news",
"bloomberg",
"business-insider",
"cbs-news",
"cnbc",
"cnn",
"espn",
"financial-times",
"fox-news",
"fox-sports",
"msnbc",
"nbc-news",
"nbc-sports",
"politico",
"reuters",
"the-hill",
"the-wall-street-journal",
"the-washington-post",
"techcrunch",
"usa-today"
]
yesterday = datetime.now() - timedelta(1)
home = Path.home()
try:
    with open(f'{home}/spinach/rag-parsing/config.yaml', 'r') as f:
        data = yaml.load(f, Loader=yaml.SafeLoader)
    model = data.get("model")
except FileNotFoundError:
    with open(f'{home}/spinach/rag-parsing/config-default.yaml', 'r') as f:
        data = yaml.load(f, Loader=yaml.SafeLoader)
messages = []
messages = [
    {
        "role": "system",
        "content": (
            "Do not repeat yourself"
            "Do not repeat back the prompt that was given to you"
            "You're a helpful assistant"
            "when asked for a quiz, do not give the answers instead give answers when grading the user's answers"
        ),
    }
]

while True:
    prompt = input('>>')
    print('')
    if prompt == '+++':
        while True:
            text = sys.stdin.readline()
            if text.strip().endswith('END'):
                break
            else:
                prompt += text
    if prompt.strip().split()[0].lower() == 'look':
        try:
            if prompt.strip().split()[1].lower() == 'dyn':
                path_loaded = None
                file_output = look(context=prompt, folder='dynamic')
                file_loaded = file_output[0]
                top_idx = file_output[1]
                prompt = file_output[2]
                if file_output is None:
                    continue
                else:
                    message_return = {"role":"user","content": f"Here is the content of the file \n```\n{file_output[0][top_idx]['content']}\n```"}
            elif prompt.strip().split()[1].lower() == 'data':
                path_loaded = None
                file_output = look(context=prompt, folder='data')
                print(file_output)
                file_loaded = file_output[0]
                top_idx = file_output[1]
                prompt = file_output[2]
                if file_output is None:
                    continue
                else:
                    message_return = {"role":"user","content": f"Here is the content of the file \n```\n{file_output[0][top_idx]['content']}\n```"}
            else:
                file_output = look(context=prompt,folder=None)
                path_loaded = prompt.strip().split()[1].lower()
                file_loaded = file_output[0]
                top_idx = file_output[1]
                prompt = file_output[2]

                if file_output is None:
                    continue
                else:
                    message_return = {"role":"user","content": f"Here is the content of the file `{path_loaded}`:\n```\n{file_output[0][top_idx]['content']}\n```"}
            messages.append(message_return)
        except TypeError as e:
            print(e)
            messages.pop()
            continue
    elif prompt.strip().split()[0].lower() == 'quit' or prompt.strip().split()[0].lower() == 'bye':
        quit()
    elif prompt.strip().split()[0].lower() == 'news':
        news = None
        if len(prompt.strip().split()) < 2:
            news = req_news(None, None, None)
            if not news:
                print('No news returned, check news api key in config-default.yaml')
                print('This tool uses NewsAPI  for retrieving news')
                print('If you do not have an api key you can retrieve one here: https://newsapi.org/')
                messages.pop()
                continue
            else:
                message_news = ''
                for key in news:
                    message_news += key + ' '
                    for i in range(len(news[key])):
                        message_news += news[key][i]
                message_return = {"role": "user",
                                    "content": f"Here is the content of the news \n```\n{message_news}\n```"}
                messages.append(message_return)
                prompt = f'summarize, at the end of each summary supply the url of article. each article should be counted. Example 1. First article 2. Second Article. Articles are from Date: {yesterday}. Summarize them in as much detail as you can while making sure to not make anything up.'
        elif len(prompt.strip().split()) == 2 and prompt.strip().split()[1] not in us_sources and prompt.strip().split()[1]!='help' and prompt.strip().split()[1]!='search':
            news = req_news(None, None, int(prompt.strip().split()[1]))

            if not news:
                print('No news returned, check news api key in config-default.yaml')
                print('This tool uses NewsAPI  for retrieving news')
                print('If you do not have an api key you can retrieve one here: https://newsapi.org/')
                messages.pop()
                continue
            else:
                message_news = ''
                for key in news:
                    message_news += key + ' '
                    for i in range(len(news[key])):
                        message_news += news[key][i]
                message_return = {"role": "user",
                                    "content": f"Here is the content of the news \n```\n{message_news}\n```"}
                messages.append(message_return)
                prompt = f'summarize, at the end of each summary supply the url of article. each article should be counted. Example 1. First article 2. Second Article. Articles are from Date: {yesterday}. Summarize them in as much detail as you can while making sure to not make anything up.'
        elif prompt.strip().split()[1].lower() == 'help':
            print('You can query specific news sources for their top headlines by prompting "news source"')
            print('Leaving this blank will query by default associated-press, politico,the-hill, and financial-times')
            print('These can be adjusted in the config.yaml')
            print('Below are the currently available news sources')
            for i in range(len(us_sources)):
                print(f'{i+1}. {us_sources[i]}')
            continue
        elif prompt.strip().split()[1].lower() == 'search':
            news = None
            if prompt.strip().split()[2].lower() in us_sources and len(prompt.strip().split()) == 4:
                news = req_news(prompt.strip().split()[2].lower(), prompt.strip().split()[3].lower(), None)
            elif prompt.strip().split()[2].lower() in us_sources and len(prompt.strip().split()) == 5:
                news = req_news(prompt.strip().split()[2].lower(), prompt.strip().split()[3].lower(),int(prompt.strip().split()[4].lower()))
            elif prompt.strip().split()[2].lower() not in us_sources and len(prompt.strip().split()) == 3:
                news = req_news(None, prompt.strip().split()[2].lower(), None)
            elif prompt.strip().split()[2].lower() not in us_sources and len(prompt.strip().split()) == 4:
                news = req_news(None,prompt.strip().split()[2].lower(),int(prompt.strip().split()[3].lower()))
            if not news:
                print('No news returned, check news api key in config-default.yaml')
                print('This tool uses NewsAPI  for retrieving news')
                print('If you do not have an api key you can retrieve one here: https://newsapi.org/')
                messages.pop()
                continue
            else:
                message_news = ''

                for key in news:
                    message_news += key + ' '
                    for i in range(len(news[key])):
                        message_news += news[key][i]
                message_return = {"role": "user",
                                    "content": f"Here is the content of the news \n```\n{message_news}\n```"}
                messages.append(message_return)

                prompt = f'summarize, at the end of each summary supply the url of article. each article should be counted. Example 1. First article 2. Second Article. Articles are from Date: {yesterday}. Summarize them in as much detail as you can while making sure to not make anything up.'
        else:
            news = None
            if len(prompt.strip().split()) != 3:
                news = req_news(prompt.strip().split()[1].lower(), None, None)
            elif len(prompt.strip().split()) == 3:
                news = req_news(prompt.strip().split()[1].lower(), None, int(prompt.strip().split()[2].lower()))
            if not news:
                print('No news returned, check news api key in config-default.yaml')
                print('This tool uses NewsAPI  for retrieving news')
                print('If you do not have an api key you can retrieve one here: https://newsapi.org/')
                messages.pop()
                continue
            else:
                message_news = ''

                for key in news:
                    message_news += key + ' '
                    for i in range(len(news[key])):
                        message_news += news[key][i]
                message_return = {"role": "user",
                                    "content": f"Here is the content of the news \n```\n{message_news}\n```"}
                messages.append(message_return)

                prompt = f'summarize, at the end of each summary supply the url of article. each article should be counted. Example 1. First article 2. Second Article. Articles are from Date: {yesterday}. Summarize them in as much detail as you can while making sure to not make anything up.'

    elif prompt.strip().split()[0].lower() == 'search':
        search_var = search(prompt.split(' ',1)[1])
        if not search_var:
            print('No search results returned, check search_api key in config-default.yaml')
            print('This tool uses Serper search API for searching google')
            print('If you do not have an api key you can retrieve one here: https://serper.dev/')
            messages.pop()
            continue
        else:
            search_results = ''

            for key in search_var:
                search_results += key + ' '
                for i in range(len(search_var[key])):
                    search_results += search_var[key][i]
            message_return = {"role": "user",
                                "content": f"Here is the content of the search results \n```\n{search_results}\n```"}
            messages.append(message_return)

            prompt = f'from the search results given, try your best to answer the question, provide sources, and if you feel as though you cannot discern a proper answer simply return the urls with a summary of each. The question or search was: {prompt.split(' ',1)[1]}'

    elif prompt.strip().split()[0].lower() == 'update':
        if path_loaded is None:
            print('no valid file loaded')
            continue
        else:
            update_prompt = "look "+ path_loaded
            print(update_prompt)
            file_output = look(context=update_prompt,folder=None)
            file_loaded = file_output[0]
            top_idx = file_output[1]
            output = file_output[2]+ f"" + file_output[0][top_idx]['content']
            message_return = {"role": "user",
                              "content": f"{file_output[2]} here is the content of the file name {path_loaded}:{file_output[0][top_idx]['content']}"}
            messages.append(message_return)
    elif prompt.strip().split()[0].lower() == 'reset':
        messages.clear()
        file_loaded.clear()
        print('context cleared')
        continue
    messages.append({"role": "user", "content": prompt})
    response = (ollama.chat(model=model,
                            messages=messages,
                            stream=True))
    ai_response = ''
    skip = False
    for i in response:
        ai_response += i["message"]["content"]
        print(i["message"]["content"], end='', flush=True)
    messages.append({"role": "assistant", "content": ai_response})
    print('')
