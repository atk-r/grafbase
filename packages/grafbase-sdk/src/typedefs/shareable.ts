import { AuthRuleF } from '../auth'
import { AuthDefinition } from './auth'
import { CacheDefinition, FieldCacheParams, FieldLevelCache } from './cache'
import { DefaultDefinition } from './default'
import { ReferenceDefinition } from './reference'
import { ScalarDefinition } from './scalar'
import { EnumDefinition } from './enum'
import { ResolverDefinition } from './resolver'
import { JoinDefinition } from './join'
import { TagDefinition } from './tag'

/**
 * A list of field types that can hold a `@shareable` attribute.
 */
export type Shareable =
  | ScalarDefinition
  | DefaultDefinition
  | ReferenceDefinition
  | EnumDefinition<any, any>
  | TagDefinition

export class ShareableDefinition {
  private field: Shareable

  constructor(field: Shareable) {
    this.field = field
  }

  /**
   * Set the field-level auth directive.
   *
   * @param rules - A closure to build the authentication rules.
   */
  public auth(rules: AuthRuleF): AuthDefinition {
    return new AuthDefinition(this, rules)
  }

  /**
   * Attach a resolver function to the field.
   *
   * @param name - The name of the resolver function file without the extension or directory.
   */
  public resolver(name: string): ResolverDefinition {
    return new ResolverDefinition(this, name)
  }

  /**
   * Attach a join function to the field.
   *
   * @param select - The field selection string to join onto this field
   */
  public join(select: string): JoinDefinition {
    return new JoinDefinition(this, select)
  }

  /**
   * Attach a join function to the field.
   *
   * @param select - The field selection string to join onto this field
   */
  public tag(tag: string): TagDefinition {
    return new TagDefinition(this, tag)
  }

  /**
   * Set the field-level cache directive.
   *
   * @param params - The cache definition parameters.
   */
  public cache(params: FieldCacheParams): CacheDefinition {
    return new CacheDefinition(this, new FieldLevelCache(params))
  }

  public toString(): string {
    return `${this.field} @shareable`
  }
}
