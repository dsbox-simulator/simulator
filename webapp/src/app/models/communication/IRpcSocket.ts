export interface IRpcSocket{
    call(method: string, params: any[]): Promise<any> ;
}