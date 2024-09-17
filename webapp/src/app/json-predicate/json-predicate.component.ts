import { CommonModule } from '@angular/common';
import { Component } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { LinkedPredicate } from './Models/LinkedPredicate';
import { PredicateStore } from './Models/PredicateStore';

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

/**
 * JsonPredicateComponent is a component to enter an test Breakpoints
 */
export class JsonPredicateComponent {
  jsonInput: string = "{\n" +
  "  \"data\": {\n" +
  "    \"msg\": {\n" +
  "      \"body\": {\n" +
  "        \"lamport\": 24,\n" +
  "        \"type\": \"store_ok\"\n" +
  "      },\n" +
  "      \"dest\": \"c1\",\n" +
  "      \"src\": \"s1\"\n" +
  "    },\n" +
  "    \"type\": \"send_message\"\n" +
  "  }\n" +
  "}";

  predicates: { id: number, value: string }[] = [{ id: 0, value: '' }];
  linkedPredicates: LinkedPredicate[] = [];
  results: string[] = [];
  predicateIdCounter: number = 1;

  /**
   * Add a new Breakpoint
   */
  addPredicate() {
    this.predicates.push({ id: this.predicateIdCounter++, value: '' });
    this.linkedPredicates.push(new LinkedPredicate('')); // Add corresponding LinkedPredicate
    PredicateStore.addEvent(new LinkedPredicate(''));
  }

  /**
   * Update a existing Breakpoint
   * @param value 
   * @param index 
   */
  updatePredicate(value: string, index: number) {
    this.predicates[index].value = value;
    this.linkedPredicates[index] = new LinkedPredicate(value); // Update LinkedPredicate whenever the value changes
    PredicateStore.updateEvent(index, new LinkedPredicate(value));
  }

  deletePredicate(index: number) {
    this.predicates.splice(index, 1);
    this.linkedPredicates.splice(index, 1); // Also remove corresponding LinkedPredicate
    PredicateStore.removeEvent(index);
  }

  /**
   * Evaluate Breakpoint against JsonInput
   * @returns 
   */
  checkPredicates() {
    let jsonObj;

    try {
      jsonObj = JSON.parse(this.jsonInput);
    } catch (e) {
      this.results = ['Invalid JSON'];
      return;
    }

    this.results = this.linkedPredicates.map((linkedPredicate, index) => {
      try {
        const res = linkedPredicate.evaluate([jsonObj]);
        linkedPredicate.reset();
        return res.toString();
      } catch (e) {
        return `Predicate ${index + 1} threw an error: ${e}`;
      }
    });

    const resultsPr = this.predicates.map((predicateObj, index) => {
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

    //this.results.push(...resultsPr);
  }
}
