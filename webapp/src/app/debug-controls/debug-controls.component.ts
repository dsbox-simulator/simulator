import { Component } from '@angular/core';
import { CoreSocketFactory } from '../models/communication/CoreSocketFactory';
import { JsonRpcWebSocketClient } from '../models/communication/RpcSocket';

@Component({
  selector: 'app-debug-controls',
  standalone: true,
  imports: [],
  templateUrl: './debug-controls.component.html',
  styleUrl: './debug-controls.component.css'
})
export class DebugControlsComponent {
  CoreSocket: JsonRpcWebSocketClient;
  constructor() {
    this.CoreSocket = CoreSocketFactory.create();
  }

  public send(data: string) {
    this.CoreSocket.call(data);
  }
}
