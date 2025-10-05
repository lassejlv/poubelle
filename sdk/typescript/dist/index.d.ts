export interface Row {
    [key: string]: number | string | null;
}
export declare class PoubelleClient {
    private socket;
    private config;
    private connected;
    private buffer;
    constructor(connectionString: string);
    private parseConnectionString;
    connect(): Promise<void>;
    query(sql: string): Promise<string>;
    createTable(name: string, columns: Record<string, 'INT' | 'TEXT'>): Promise<string>;
    insert(table: string, data: Record<string, number | string | null>): Promise<string>;
    select(table: string, columns?: string[]): Promise<Row[]>;
    close(): Promise<void>;
    private send;
    private waitForPrompt;
}
export default PoubelleClient;
