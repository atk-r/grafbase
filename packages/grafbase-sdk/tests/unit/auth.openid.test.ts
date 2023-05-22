import { config, g, auth } from '../../src/index'
import { describe, expect, it, beforeEach } from '@jest/globals'

describe('OpenID auth provider', () => {
  beforeEach(() => g.clear())

  it('renders a provider with private access', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private }
          ]
        )"
    `)
  })

  it('renders a provider with custom clientId', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}',
      clientId: 'some-id'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}", clientId: "some-id" }
          ]
          rules: [
            { allow: private }
          ]
        )"
    `)
  })

  it('renders a provider with custom groupsClaim', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}',
      groupsClaim: 'admin'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}", groupsClaim: "admin" }
          ]
          rules: [
            { allow: private }
          ]
        )"
    `)
  })

  it('renders a provider with private access for get', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private().get()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private, operations: [get] }
          ]
        )"
    `)
  })

  it('renders a provider with private access for list', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private().list()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private, operations: [list] }
          ]
        )"
    `)
  })

  it('renders a provider with private access for read', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private().read()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private, operations: [read] }
          ]
        )"
    `)
  })

  it('renders a provider with private access for create', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private().create()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private, operations: [create] }
          ]
        )"
    `)
  })

  it('renders a provider with private access for update', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private().update()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private, operations: [update] }
          ]
        )"
    `)
  })

  it('renders a provider with private access for delete', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private().delete()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private, operations: [delete] }
          ]
        )"
    `)
  })

  it('renders a provider with private access for get, list and read', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private().get().list().read()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private, operations: [get, list, read] }
          ]
        )"
    `)
  })

  it('renders a provider with private access for all operations', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private().all()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private, operations: [get, list, read, create, update, delete] }
          ]
        )"
    `)
  })

  it('renders a provider with owner access', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.owner()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: owner }
          ]
        )"
    `)
  })

  it('renders a provider with owner access and custom operations', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.owner().create()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: owner, operations: [create] }
          ]
        )"
    `)
  })

  it('renders a provider with groups access', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.groups(['backend', 'admin'])
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: groups, groups: ["backend", "admin"] }
          ]
        )"
    `)
  })

  it('renders a provider with groups access and custom operations', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.groups(['backend', 'admin']).delete()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: groups, groups: ["backend", "admin"], operations: [delete] }
          ]
        )"
    `)
  })

  it('renders multiple providers like a champ', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const derp = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk, derp],
        rules: (rules) => {
          rules.private()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private }
          ]
        )"
    `)
  })

  it('renders multiple rules like a champ', () => {
    const clerk = auth.OpenIDConnect({
      issuer: '{{ env.ISSUER_URL }}'
    })

    const cfg = config({
      schema: g,
      auth: {
        providers: [clerk],
        rules: (rules) => {
          rules.private().read()
          rules.owner().create()
          rules.groups(['backend', 'admin']).delete()
        }
      }
    })

    expect(cfg.toString()).toMatchInlineSnapshot(`
      "extend schema
        @auth(
          providers: [
            { type: oidc, issuer: "{{ env.ISSUER_URL }}" }
          ]
          rules: [
            { allow: private, operations: [read] }
            { allow: owner, operations: [create] }
            { allow: groups, groups: ["backend", "admin"], operations: [delete] }
          ]
        )"
    `)
  })
})
