=begin
#UKCloud Portal API

#No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)

The version of the OpenAPI document: development

Generated by: https://openapi-generator.tech
OpenAPI Generator version: 4.2.2

=end

require 'spec_helper'
require 'json'
require 'date'

# Unit tests for PortalClient::EdgegatewayBuildRequestAttributes
# Automatically generated by openapi-generator (https://openapi-generator.tech)
# Please update as you see appropriate
describe 'EdgegatewayBuildRequestAttributes' do
  before do
    # run before each test
    @instance = PortalClient::EdgegatewayBuildRequestAttributes.new
  end

  after do
    # run after each test
  end

  describe 'test an instance of EdgegatewayBuildRequestAttributes' do
    it 'should create an instance of EdgegatewayBuildRequestAttributes' do
      expect(@instance).to be_instance_of(PortalClient::EdgegatewayBuildRequestAttributes)
    end
  end
  describe 'test attribute "connectivity_type"' do
    it 'should work' do
      # assertion here. ref: https://www.relishapp.com/rspec/rspec-expectations/docs/built-in-matchers
      # validator = Petstore::EnumTest::EnumAttributeValidator.new('String', ["Internet", "External"])
      # validator.allowable_values.each do |value|
      #   expect { @instance.connectivity_type = value }.not_to raise_error
      # end
    end
  end

end
