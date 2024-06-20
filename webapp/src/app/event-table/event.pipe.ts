import { Pipe, PipeTransform } from '@angular/core';
import { JsonRpcEvent } from '../models/communication/RpcEvent';

@Pipe({ name: 'event', standalone: true})
export class EventPipe implements PipeTransform {
  transform(values: JsonRpcEvent[], filter: string): JsonRpcEvent[] {
    if (!filter || filter.length === 0) {
      return values;
    }

    if (values.length === 0) {
      return values;
    }

    console.log("Pipes: " + values);

    return values.filter((value: JsonRpcEvent) => {

      //todo filter all
      const body = value.params?.data?.msg?.body;
      const dest = value.params?.data?.msg?.dest;

      let bodyStr = JSON.stringify(body);
      const destStr = typeof dest === 'string' ? dest.toLowerCase() : '';

      const nameFound = bodyStr.includes(filter.toLowerCase());
      const capitalFound = destStr.includes(filter.toLowerCase());

      return nameFound || capitalFound;
    });
  }
}