import psycopg2
import os
import yaml
import pandas as pd



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

    def create_table(self, table, table_name):
        cur = self.connect.cursor()
        if cur is None:
            return

        dtype_converter = {
            "int64": "bigint",
            "int32": "integer",
            "float64": "double precision",
            "float32": "real",
            "bool": "boolean",
            "boolean": "boolean",
            "object": "text",
            "string": "text",
            "datetime64[ns]": "timestamp",
            "datetime64[ns, utc]": "timestamptz",
        }
        d = ([(col, dtype_converter[str(table[col].dtype)])
             for col in list(table.columns)])

        cols = ',\n'.join([f'\t{col} {typ}' for col, typ in d])

        query = f"""
        create table {table_name}(
        {cols}
        );
        """
        cur.execute(query)
        self.connect.commit()

        cur.close()

    def upload_data(self, table, table_name):
        cur = self.connect.cursor()
        if cur is None:
            return

        buffer = StringIO()

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
