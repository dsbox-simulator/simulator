import { CommonModule } from '@angular/common';
import { Component } from '@angular/core';
import { FormsModule } from '@angular/forms';

@Component({
  selector: 'app-json-predicate',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule
  ],
  templateUrl: './json-predicate.component.html',
  styleUrls: ['./json-predicate.component.scss']
})
export class JsonPredicateComponent {
  jsonInput: string = '{'+
'"test":1,'+
'"test2":"value"'+
'}';
  predicates: { id: number, value: string }[] = [{ id: 0, value: '' }];
  results: string[] = [];
  predicateIdCounter: number = 1;

  addPredicate() {
    this.predicates.push({ id: this.predicateIdCounter++, value: '' });
  }

  checkPredicates() {
    let jsonObj;

    try {
      jsonObj = JSON.parse(this.jsonInput);
    } catch (e) {
      this.results = ['Invalid JSON'];
      return;
    }

    this.results = this.predicates.map((predicateObj, index) => {
      let predicate: (obj: any) => boolean;
      try {
        predicate = new Function('obj', `return (${predicateObj.value})(obj)`) as (obj: any) => boolean;
      } catch (e) {
        return `Predicate ${index + 1} is invalid ${predicateObj.value}`;
      }

      try {
        const res = predicate(jsonObj);
        return res.toString();
      } catch (e) {
        return `Predicate ${index + 1} threw an error: ${e}`;
      }
    });
  }
}
