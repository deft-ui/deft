import * as sqliteConnApi from "native";

export class SqliteConn {
    #conn;
    constructor(conn) {
        this.#conn = conn;
    }

    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<number>}
     */
    async execute(sql, params= []) {
        return await sqliteConnApi.execute(this.#conn, sql, params);
    }

    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<Object[]>}
     */
    async query(sql, params = []) {
        const [columnNames, rows] = await sqliteConnApi.query(this.#conn, sql, params);
        return rows.map(it => {
            const map = {};
            for (let i = 0; i < columnNames.length; i++) {
                map[columnNames[i]] = it[i];
            }
            return map;
        });
    }

}

export class Sqlite {

    /**
     *
     * @param path {string}
     * @returns {Promise<SqliteConn>}
     */
    static async open(path) {
        const conn = sqliteConnApi.create();
        await sqliteConnApi.open(conn, path);
        return new SqliteConn(conn);
    }

}