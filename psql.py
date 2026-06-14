import psycopg2
import os
import yaml
import pandas as pd
from io import StringIO



class psql:
    def __init__(self):
        # os.chdir('..')
        # base_dir = os.path.dirname(os.path.abspath(__file__))
        # print(base_dir)
        # config = os.path.join(base_dir, 'config.yaml')
        # print(config)
        with open('config.yaml', 'r') as file:
            config = yaml.safe_load(file)

        try:
            print("database connection successful")
            self.connect = (psycopg2.connect(database=config['database'],
                                             user=config['user'],
                                             password=config['password'],
                                             host=config['host']))
        except psycopg2.operationalerror:
            print("database connection failed")
            return None


    def upload_data(self, df, table_name):
        print("reached")
        cur = self.connect.cursor()
        if cur is None:
            return

        buffer = StringIO()
        table = pd.DataFrame(df)

        table.to_csv(buffer, index=False, header=False)

        buffer.seek(0)

        cols = ',\n'.join([f'\t{col}' for col in table.columns])

        cur.copy_expert(
                f"""copy {table_name}
                ({cols})
                from stdin with (format csv)""", buffer)

        self.connect.commit()

    def query(self, query):
        cur = self.connect.cursor()

        cur.execute(query)
        columns = [desc[0] for desc in cur.description]
        data = pd.DataFrame(cur.fetchall(), columns=columns)

        return data

    def close(self):

        self.connect.close()


if __name__ == '__main__':
    psql()
